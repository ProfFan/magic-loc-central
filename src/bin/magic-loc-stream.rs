use binrw::BinRead;
use futures::{
    future::{join, ready},
    stream::FuturesUnordered,
    StreamExt,
};
use magic_loc_central::{proto, stream_decoder::MagicLocStreamDecoder};
use tokio;
use tokio_util::codec::Decoder;

use std::{collections::VecDeque, time::Duration};

use clap::Parser;

use serde_json;

// tracing
use tracing::{debug, error, info, trace};

// serialport
use serialport_low_latency;
use tokio_serial::{self, SerialPort, SerialPortBuilderExt, SerialStream};

#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
pub struct Options {
    /// Increase verbosity, and can be used multiple times
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Serial port devices
    #[arg(short, long, required = true, num_args = 1..)]
    pub serial_ports: Vec<String>,
}

#[tokio::main]
pub async fn main() {
    // Parse command line
    let opts = Options::parse();

    let debug_level = match opts.verbose {
        0 => tracing::Level::INFO,
        1 => tracing::Level::DEBUG,
        _ => tracing::Level::TRACE,
    };
    tracing_subscriber::fmt().with_max_level(debug_level).init();

    info!("Starting with options: {:?}", opts);

    // Open the supplied serial ports
    let mut serial_ports = Vec::new();
    for port in opts.serial_ports {
        let serial_port = tokio_serial::new(port.to_owned(), 921600).open_native();
        let mut serial_port = serial_port.unwrap();

        // Set the serial port to low latency mode
        serialport_low_latency::enable_low_latency(&mut serial_port).unwrap();

        drop(serial_port);

        let serial_port = tokio_serial::new(port, 2000000)
            .timeout(Duration::from_millis(10))
            .open_native_async()
            .unwrap();

        serial_ports.push(serial_port);
    }

    let mut readers = Vec::new();
    for serial_port in serial_ports {
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
            // CirReport
            if &decoded[0..3] == b"CIR".as_slice() {
                // Use binrw to decode the packet
                let decoded =
                    proto::CirReport::read(&mut binrw::io::Cursor::new(&decoded[..])).unwrap();

                let cir_report = proto::ConvertedCirReport::from(decoded);

                // print the decoded packet
                debug!("Decoded packet from {}: {:?}", id, cir_report);

                // Convert to JSON
                let json = serde_json::to_string(&cir_report).unwrap();
                println!("{}", json);
            } else {
                error!("Unknown packet: {:?}", decoded);
            }
        }

        // Re-add the reader to the packet_futures
        packet_futures.push(join(ready(id), reader.into_future()));
    }
}
