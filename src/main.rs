use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
};

use binrw::{BinRead, BinWrite};
use futures::{
    future::{join, ready},
    stream::FuturesUnordered,
    SinkExt, StreamExt,
};
use nalgebra::Vector3;
use stream_decoder::MagicLocStreamDecoder;
use tmq::{self, publish, Context};
use tokio;
use tokio_serial::{self, SerialPort, SerialPortBuilderExt, SerialStream};
use tokio_util::codec::Decoder;
use tracing::{debug, error, info, trace};

use rzcobs;

// Command line parser
mod command_line;
// Protocol definitions
mod proto;
// Async stream decoder for the custom wire format.
mod stream_decoder;
// Optimization for the location of the device
mod optimization;

mod configuration;

/// Synchronize the incoming packets according to the sequence number
///
/// This function is called when a new packet arrives from a serial port.
pub fn synchronize(
    serial_fifos: &mut Vec<VecDeque<proto::RangeReport>>,
) -> Option<Vec<proto::RangeReport>> {
    // Check if all the FIFO queues are non-empty
    for fifo in serial_fifos.iter() {
        if fifo.is_empty() {
            return None;
        }
    }

    let mut txts_count = HashMap::<u64, usize>::new();
    for fifo in serial_fifos.iter() {
        for report in fifo.iter() {
            let count = txts_count.entry(report.trigger_txts).or_insert(0);
            *count += 1;
        }
    }

    // If any of the TXTS is present in all the FIFO queues, then we have a match
    // We drop all the previous packets and return the matched packets
    let txts_match = txts_count
        .iter()
        .find(|(_, &count)| count == serial_fifos.len());

    if txts_match.is_none() {
        return None;
    }

    let txts_match = txts_match.unwrap().0;

    // drop all the previous packets until the TXTS match
    for fifo in serial_fifos.iter_mut() {
        while let Some(front) = fifo.front() {
            if front.trigger_txts == *txts_match {
                break;
            }

            fifo.pop_front();
        }
    }

    // Now all the FIFO queues have the same TXTS at the front
    // We can return the packets
    let mut packets = Vec::new();
    for fifo in serial_fifos.iter_mut() {
        packets.push(fifo.pop_front().unwrap());
    }

    Some(packets)
}

/// Synchronize the incoming packets according to the sequence number
/// and publish the synchronized packets to the ZMQ publisher
pub async fn sync_and_publish(
    mut publisher: tmq::publish::Publish,
    serial_ports: Vec<SerialStream>,
) {
    // Create FIFO queue for all the serial ports
    let mut serial_fifos: Vec<VecDeque<proto::RangeReport>> = Vec::new();
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

            // Add the packet to the FIFO queue
            serial_fifos[id].push_back(decoded);

            // Synchronize the packets
            while let Some(mut packets) = synchronize(&mut serial_fifos) {
                // print the synchronized packets
                debug!("Synchronized packets: {:?}", packets);

                for packet in packets.iter_mut() {
                    packet.ranges.iter_mut().for_each(|x| *x -= 76.8);
                }

                debug!("Bias subtracted: {:?}", packets);

                // Publish the synchronized packets
                let buf = Vec::new();
                let mut writer = binrw::io::Cursor::new(buf);
                packets.write(&mut writer).unwrap();
                let result = publisher
                    .send(vec![b"ranges".to_vec(), writer.into_inner()])
                    .await;
                if result.is_err() {
                    error!("Error publishing to ZMQ: {:?}", result);
                }

                // Localize
                let mut locations = Vec::new();
                for packet in packets.iter_mut() {
                    let distances = packet.ranges;
                    let point = optimization::localize_point(&distances);

                    // Convert to [f64; 3]
                    let point = point.unwrap_or_else(|| Vector3::new(0.0, 0.0, 0.0));
                    let point = [point[0] as f64, point[1] as f64, point[2] as f64];
                    locations.push(point);
                }

                // send the locations to the publisher as JSON
                let json = serde_json::to_string(&locations).unwrap();
                let _ = publisher
                    .send(vec![b"points".to_vec(), json.into_bytes()])
                    .await;

                info!("Locations: {:0.2?}", locations);
            }
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
    let publisher = tmq::publish(&Context::new()).bind(&opts.zmq_addr).unwrap();

    // Open the supplied serial ports
    let mut serial_ports = Vec::new();
    for port in opts.serial_ports {
        let serial_port = tokio_serial::new(port, 921600).open_native_async();
        serial_ports.push(serial_port.unwrap());
    }

    // synchronize and publish the packets
    sync_and_publish(publisher, serial_ports).await;
}
