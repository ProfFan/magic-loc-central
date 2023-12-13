use std::{cell::RefCell, collections::VecDeque};

use binrw::BinRead;
use futures::{
    future::{join, ready},
    stream::FuturesUnordered,
    StreamExt,
};
use stream_decoder::MagicLocStreamDecoder;
use tmq::{self, publish, Context};
use tokio;
use tokio_serial::{self, SerialPort, SerialPortBuilderExt, SerialStream};
use tokio_util::codec::Decoder;
use tracing::{debug, info, trace};

use rzcobs;

// Command line parser
mod command_line;
// Protocol definitions
mod proto;
// Async stream decoder for the custom wire format.
mod stream_decoder;

/// Synchronize the incoming packets according to the sequence number
/// and publish the synchronized packets to the ZMQ publisher
pub async fn sync_and_publish(
    mut publisher: tmq::publish::Publish,
    serial_ports: Vec<SerialStream>,
) {
    // Create FIFO queue for all the serial ports
    let mut serial_fifos = Vec::new();
    let mut readers = Vec::new();
    for serial_port in serial_ports {
        serial_fifos.push(VecDeque::<proto::RangeReport>::new());
        readers.push(MagicLocStreamDecoder.framed(serial_port).boxed());
    }

    // Listen to all the serial ports
    let mut packet_futures = FuturesUnordered::new();
    for (id, reader) in readers.iter_mut().enumerate() {
        packet_futures.push(join(ready(id), reader.into_future()));
    }

    loop {
        // Wait for the next packet to arrive (from any serial port)
        let (id, (packet, reader)) = packet_futures.next().await.unwrap();

        // Decode the packet
        let result = packet.unwrap();
        if result.is_err() {
            panic!("Error reading from serial port: {:?}", result);
        }

        let packet = result.unwrap();

        // print the packet
        trace!("Packet from {}: {:?}", id, packet);

        let decoded = rzcobs::decode(&packet[4..]);

        if let Ok(decoded) = decoded {
            // Use binrw to decode the packet
            let decoded =
                proto::RangeReport::read(&mut binrw::io::Cursor::new(&decoded[..])).unwrap();

            // print the decoded packet
            debug!("Decoded packet from {}: {:?}", id, decoded);
        } else {
            debug!("Decoding error: {:?}", decoded);
        }

        // add a new future waiting for the next packet
        packet_futures.push(join(ready(id), reader.into_future()));
    }
}

#[tokio::main]
pub async fn main() {
    println!("Main thread started");

    // Parse command line
    let opts = command_line::parse();

    info!("Starting with options: {:?}", opts);

    // Open zmq publisher
    let mut publisher = tmq::publish(&Context::new()).bind(&opts.zmq_addr).unwrap();

    // Open the supplied serial ports
    let mut serial_ports = Vec::new();
    for port in opts.serial_ports {
        let serial_port = tokio_serial::new(port, 921600).open_native_async();
        serial_ports.push(serial_port.unwrap());
    }

    // synchronize and publish the packets
    sync_and_publish(publisher, serial_ports).await;
}
