// This module implements FNV-1a hashing algorithm
// http://www.isthe.com/chongo/tech/comp/fnv/

const FNV_128_PRIME: u128 = 309485009821345068724781371; // 2^88 + 2^8 + 0x3b
const FNV_128_OFFSET_BASIS: u128 = 144066263297769815596495629667062367629;

pub fn fnv1a_128<D: AsRef<[u8]>>(data: D) -> u128 {
    let mut hash = FNV_128_OFFSET_BASIS;

    for byte in data.as_ref() {
        hash ^= *byte as u128;
        hash = hash.wrapping_mul(FNV_128_PRIME);
    }

    hash
}

#[cfg(test)]
mod tests {
    use super::fnv1a_128;

    #[test]
    fn digest() {
        assert_eq!(
            fnv1a_128("Hello World!"),
            0xd2d42892ede872031d2593366229c2d2
        );

        assert_eq!(
            fnv1a_128("Hello world!"),
            0x3c94fff9ede872031d95566a45770eb2
        );

        assert_eq!(fnv1a_128("🦄🌈"), 0xa25841ae4659905b36cb0d359fad39f);
    }
}
