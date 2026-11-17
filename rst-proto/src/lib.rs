//! rst-proto: the RilStream wire format. See SPEC.md.
#![no_std]

pub mod cobs;
pub mod crc;
pub mod descriptor;
pub mod frame;

/// Protocol version carried in every frame (SPEC.md section 5).
pub const PROTOCOL_VERSION: u8 = 1;
/// Header: version, channel, schema_id, seq (2), t_us (4).
pub const HEADER_LEN: usize = 9;
pub const CRC_LEN: usize = 2;
/// Maximum decoded frame size (SPEC.md section 4).
pub const MAX_FRAME_LEN: usize = 120;
/// Channel 0 carries descriptors.
pub const DESCRIPTOR_CHANNEL: u8 = 0;
