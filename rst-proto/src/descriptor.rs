//! Field types and channel descriptors. See SPEC.md sections 6 and 7.

/// Field type codes (SPEC.md section 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FieldType {
    U8 = 0x01,
    I8 = 0x02,
    U16 = 0x03,
    I16 = 0x04,
    U32 = 0x05,
    I32 = 0x06,
    F32 = 0x07,
}

impl FieldType {
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0x01 => Some(Self::U8),
            0x02 => Some(Self::I8),
            0x03 => Some(Self::U16),
            0x04 => Some(Self::I16),
            0x05 => Some(Self::U32),
            0x06 => Some(Self::I32),
            0x07 => Some(Self::F32),
            _ => None,
        }
    }

    /// Size in bytes on the wire.
    pub const fn size(self) -> usize {
        match self {
            Self::U8 | Self::I8 => 1,
            Self::U16 | Self::I16 => 2,
            Self::U32 | Self::I32 | Self::F32 => 4,
        }
    }
}

// TODO: Descriptor encode/decode, schema_id computation.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_round_trip() {
        for code in 0x01..=0x07u8 {
            assert_eq!(FieldType::from_code(code).unwrap() as u8, code);
        }
        assert_eq!(FieldType::from_code(0x00), None);
        assert_eq!(FieldType::from_code(0x08), None);
    }

    #[test]
    fn sizes() {
        assert_eq!(FieldType::I16.size(), 2);
        assert_eq!(FieldType::I32.size(), 4);
        assert_eq!(FieldType::U8.size(), 1);
    }
}
