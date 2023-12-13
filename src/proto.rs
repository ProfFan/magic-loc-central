// Communication with the host computer

use binrw::{io::Cursor, *};

#[derive(Default, Debug, PartialEq, Clone, Copy)]
#[binrw]
#[brw(magic = b"RNG", little)]
pub struct RangeReport {
    pub seq_num: u8,
    pub tag_addr: u16,
    pub trigger_txts: u64,
    pub ranges: [f64; 8],
}
