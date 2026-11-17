//! CRC-16/CCITT-FALSE: poly 0x1021, init 0xFFFF, no reflection, xorout 0.

pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SPEC.md section 12.1
    #[test]
    fn check_value() {
        assert_eq!(crc16(b"123456789"), 0x29B1);
    }

    /// SPEC.md section 12.3: the schema_id is the low byte of this CRC.
    #[test]
    fn enc_descriptor_payload() {
        let payload = [
            0x02, 0x32, 0x00, 0x03, 0x65, 0x6E, 0x63, 0x02, 0x06, 0x00, 0x00, 0x80, 0x3F, 0x01,
            0x6C, 0x04, 0x74, 0x69, 0x63, 0x6B, 0x06, 0x00, 0x00, 0x80, 0x3F, 0x01, 0x72, 0x04,
            0x74, 0x69, 0x63, 0x6B,
        ];
        assert_eq!(crc16(&payload), 0xDCB6);
    }
}
