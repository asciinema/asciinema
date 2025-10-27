pub fn encode<N: Into<u64>>(value: N) -> Vec<u8> {
    let mut value: u64 = value.into();
    let mut bytes = Vec::new();

    while value > 127 {
        let mut low = value & 127;
        value >>= 7;

        if value > 0 {
            low |= 128;
        }

        bytes.push(low as u8);
    }

    if value > 0 || bytes.is_empty() {
        bytes.push(value as u8);
    }

    bytes
}

#[cfg(test)]
mod tests {
    use super::encode;

    #[test]
    fn test_encode() {
        assert_eq!(encode(0u64), [0x00]);
        assert_eq!(encode(1u64), [0x01]);
        assert_eq!(encode(127u64), [0x7F]);
        assert_eq!(encode(128u64), [0x80, 0x01]);
        assert_eq!(encode(255u64), [0xFF, 0x01]);
        assert_eq!(encode(256u64), [0x80, 0x02]);
        assert_eq!(encode(16383u64), [0xFF, 0x7F]);
        assert_eq!(encode(16384u64), [0x80, 0x80, 0x01]);
    }
}
