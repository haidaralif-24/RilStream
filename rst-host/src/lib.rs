//! rst-host: host-side library for RilStream. See SPEC.md sections 8 and 9.

pub use rst_proto as proto;

use std::io;

/// A source of raw bytes (UDP datagrams, serial, or an in-memory mock).
///
/// TODO: UdpTransport, SerialTransport, MockTransport (SPEC.md section 8.4).
pub trait Transport {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
}
