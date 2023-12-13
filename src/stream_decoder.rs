// Async stream decoder for the custom wire format.
//
// The decoder is implemented as a state machine. The state machine performs:
// 1. Get the currently available bytes from the serial port
// 2. Drop all bytes until a zero byte is found
// 3. Push all received bytes into the buffer until another zero byte is found
// 4. Check if the first two bytes are the header bytes [0xFF, 0x01]
// 5. If the header bytes are found, return the buffer slice

use tokio_util::{
    bytes::{Buf, BytesMut},
    codec::Decoder,
};

pub struct MagicLocStreamDecoder;

impl Decoder for MagicLocStreamDecoder {
    type Item = Vec<u8>;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Get the currently available bytes from the serial port
        let available_bytes = src.len();
        if available_bytes == 0 {
            return Ok(None);
        }

        // Drop all bytes until a zero byte is found
        let mut zero_byte_found = false;
        let mut zero_byte_index = 0;
        for (index, byte) in src.iter().enumerate() {
            if *byte == 0 {
                zero_byte_found = true;
                zero_byte_index = index;
                break;
            }
        }
        if !zero_byte_found {
            src.clear();
            return Ok(None);
        }
        src.advance(zero_byte_index); // Drop all bytes until the zero byte

        // at this point, the first byte is a zero byte
        // Check if the header bytes are found
        if src.len() < 4 {
            return Ok(None);
        }

        // Check if the first three non-zero bytes are the header bytes [0xFF, 0x01, 0x00]
        if src[1] != 0xFF || src[2] != 0x01 || src[3] != 0x00 {
            // This is not a valid packet, drop til the next zero byte
            let mut zero_byte_found = false;
            let mut zero_byte_index = 0;
            for (index, byte) in src.iter().enumerate() {
                if *byte == 0 && index > 0 {
                    zero_byte_found = true;
                    zero_byte_index = index;
                    break;
                }
            }
            if !zero_byte_found {
                src.clear();
                return Ok(None);
            } else {
                src.advance(zero_byte_index); // Drop all bytes until the zero byte
            }
            return Ok(None);
        }

        // Push all received bytes into the buffer until another zero byte is found
        let mut zero_byte_found = false;
        let mut zero_byte_index = 0;
        for (index, byte) in src.iter().skip(4).enumerate() {
            if *byte == 0 {
                zero_byte_found = true;
                zero_byte_index = index;
                break;
            }
        }
        if !zero_byte_found {
            return Ok(None);
        }

        // At this point, we have a buffer with 0x00FF0100...0x00
        let result = src.split_to(zero_byte_index + 4);
        Ok(Some(result.to_vec()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decoder() {
        let mut decoder = MagicLocStreamDecoder {};
        let mut buffer = BytesMut::new();

        // Test 1: empty buffer
        let result = decoder.decode(&mut buffer);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        // Test 2: buffer with valid packet
        let mut buffer = BytesMut::new();
        buffer.extend_from_slice(&[0x00, 0xFF, 0x01, 0x00, 0x02, 0x03, 0x00]);
        let result = decoder.decode(&mut buffer);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().unwrap(),
            [0x00, 0xFF, 0x01, 0x00, 0x02, 0x03]
        );

        // Test 3: buffer with invalid start bytes
        let mut buffer = BytesMut::new();
        buffer.extend_from_slice(&[0x11, 0x22, 0x00, 0x01, 0x02, 0x03, 0x00]);
        buffer.extend_from_slice(&[0x00, 0xFF, 0x01, 0x00, 0x02, 0x03, 0x00]);
        let result = decoder.decode(&mut buffer);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
        assert_eq!(&buffer[..], b"\0\0\xff\x01\x00\x02\x03\0");

        let _ = decoder.decode(&mut buffer);
        assert_eq!(&buffer[..], b"\0\xff\x01\x00\x02\x03\0");

        let result = decoder.decode(&mut buffer);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().unwrap(),
            [0x00, 0xFF, 0x01, 0x00, 0x02, 0x03]
        );

        // Test 4: buffer with invalid bytes and one less zero
        let mut buffer = BytesMut::new();
        buffer.extend_from_slice(&[0x11, 0x22, 0x00, 0x01, 0x02, 0x03]);
        buffer.extend_from_slice(&[0x00, 0xFF, 0x01, 0x00, 0x02, 0x03, 0x00]);
        let result = decoder.decode(&mut buffer);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
        assert_eq!(&buffer[..], b"\0\xff\x01\x00\x02\x03\0");

        let result = decoder.decode(&mut buffer);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().unwrap(),
            [0x00, 0xFF, 0x01, 0x00, 0x02, 0x03]
        );
        assert_eq!(&buffer[..], [0u8]);
    }

    #[test]
    fn test_real_data() {
        let mut decoder = MagicLocStreamDecoder {};

        let mut buffer = BytesMut::new();
        buffer.extend_from_slice(&[
            0x0, 0x30, 0x12, 0x53, 0x6a, 0x7f, 0x0, 0xff, 0x0, 0x45, 0x7e, 0x0, 0xff, 0x1, 0x0,
            0x52, 0x4e, 0x47, 0x34, 0x1, 0x60, 0xf0, 0xff, 0x1f, 0xf0, 0x3f, 0xff, 0x7e, 0xf0,
            0xff, 0x7c, 0xf0, 0xff, 0x79, 0xf0, 0xff, 0x73, 0xf0, 0xff, 0x67, 0xf0, 0xff, 0x4f,
            0xf0, 0xff, 0x1f, 0x0, 0x0, 0xff, 0x0, 0x30, 0x12, 0x59, 0x6a, 0x7f, 0x0, 0xff, 0x0,
            0x45, 0x7e, 0x0, 0xff, 0x0, 0x32, 0xd, 0x59, 0xea, 0xc5, 0xa, 0xa, 0x5f, 0x6c, 0x7e,
            0x0, 0xff, 0x1, 0x0, 0x52, 0x4e, 0x47, 0x99, 0x1, 0xa4, 0x20, 0xb5, 0xcd, 0x11, 0xa8,
            0x95, 0x53, 0x40, 0xda, 0x5f, 0x25, 0xe5, 0x98, 0x7a, 0x55, 0x40, 0x5, 0xa5, 0x2a,
            0xc6, 0x15, 0x23, 0x54, 0x40, 0xb0,
        ]);

        let result = decoder.decode(&mut buffer);
        assert!(result.is_ok());

        let _ = decoder.decode(&mut buffer);
        let result = decoder.decode(&mut buffer);
        // assert!(result.is_ok());
        println!("{:?}", result);
        println!("{:?}", buffer);

        let result = decoder.decode(&mut buffer);
        assert_eq!(
            result.unwrap().unwrap(),
            &[
                0, 255, 1, 0, 82, 78, 71, 52, 1, 96, 240, 255, 31, 240, 63, 255, 126, 240, 255,
                124, 240, 255, 121, 240, 255, 115, 240, 255, 103, 240, 255, 79, 240, 255, 31
            ]
        );

        let _ = decoder.decode(&mut buffer);
        let _ = decoder.decode(&mut buffer);
    }
}
