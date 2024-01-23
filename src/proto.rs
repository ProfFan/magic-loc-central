// Communication with the host computer

use binrw::*;
use num::complex::Complex;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[binrw]
#[brw(magic = b"RNG", little)]
pub struct RangeReport {
    pub tag_addr: u16,
    pub system_ts: u64,
    pub seq_num: u8,
    pub trigger_txts: u64,
    pub ranges: [f64; 8],
}

#[derive(Default, Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[binrw]
#[brw(magic = b"IMU", little)]
pub struct ImuReport {
    pub tag_addr: u16,
    pub system_ts: u64,
    pub accel: [u32; 3],
    pub gyro: [u32; 3],
}

#[derive(Default, Debug, PartialEq, Clone, Copy)]
#[binrw]
#[repr(C)]
pub struct RawCirSample {
    pub real: [u8; 3],
    pub imag: [u8; 3],
}

// Assert size of RawCirSample
const _: [(); 6] = [(); core::mem::size_of::<RawCirSample>()];

#[derive(Debug, Clone, Copy)]
#[binrw]
#[brw(magic = b"CIR", little)]
pub struct CirReport {
    pub src_addr: u16,
    pub system_ts: u64,
    pub seq_num: u8,
    pub ip_poa: u16,      // Phase of Arrival
    pub fp_index: u16,    // First Path Index
    pub start_index: u16, // Start Index of CIR
    pub cir_size: u16,
    pub cir: [RawCirSample; 16],
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ConvertedCirReport {
    pub src_addr: u16,
    pub system_ts: u64,
    pub seq_num: u8,
    pub ip_poa: u16,      // Phase of Arrival
    pub fp_index: u16,    // First Path Index
    pub start_index: u16, // Start Index of CIR
    pub cir_size: u16,
    pub cir: [Complex<f64>; 16],
}

impl From<CirReport> for ConvertedCirReport {
    fn from(report: CirReport) -> Self {
        let mut cir = [Complex::new(0.0, 0.0); 16];

        for i in 0..16 {
            let real = report.cir[i].real;
            let imag = report.cir[i].imag;

            let real = i32::from_le_bytes([
                real[0],
                real[1],
                real[2],
                if real[2] & 0x80 == 0x80 { 0xFF } else { 0x00 },
            ]);
            let imag = i32::from_le_bytes([
                imag[0],
                imag[1],
                imag[2],
                if imag[2] & 0x80 == 0x80 { 0xFF } else { 0x00 },
            ]);

            cir[i] = Complex::new(real as f64, imag as f64);
        }

        ConvertedCirReport {
            src_addr: report.src_addr,
            system_ts: report.system_ts,
            seq_num: report.seq_num,
            ip_poa: report.ip_poa,
            fp_index: report.fp_index,
            start_index: report.start_index,
            cir_size: report.cir_size,
            cir,
        }
    }
}
