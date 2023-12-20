// Communication with the host computer

use binrw::*;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[binrw]
#[brw(magic = b"RNG", little)]
pub struct RangeReport {
    pub seq_num: u8,
    pub tag_addr: u16,
    pub trigger_txts: u64,
    pub ranges: [f64; 8],
}
