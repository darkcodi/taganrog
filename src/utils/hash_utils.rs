use uuid::Uuid;

pub struct MurMurHasher;

impl MurMurHasher {
    pub fn hash_str(str: &str) -> String {
        MurMurHasher::hash_bytes(str.as_bytes())
    }

    pub fn hash_bytes(bytes: &[u8]) -> String {
        let hash = fastmurmur3::murmur3_x64_128(bytes, 0);
        let guid = Uuid::from_bytes(hash.to_le_bytes());
        guid.to_string().to_lowercase().replace('-', "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_str() {
        assert_eq!(MurMurHasher::hash_str("hello"), "029bbd41b3a7d8cb191dae486a901e5b");
        assert_eq!(MurMurHasher::hash_str("world"), "ea84fbf00a79c5713a8e3571c3ece4c4");
        assert_eq!(MurMurHasher::hash_str("hello world"), "0e617feb46603f53b163eb607d4697ab");
    }

    #[test]
    fn test_hash_bytes() {
        assert_eq!(MurMurHasher::hash_bytes(b"hello"), "029bbd41b3a7d8cb191dae486a901e5b");
        assert_eq!(MurMurHasher::hash_bytes(b"world"), "ea84fbf00a79c5713a8e3571c3ece4c4");
        assert_eq!(MurMurHasher::hash_bytes(b"hello world"), "0e617feb46603f53b163eb607d4697ab");
    }
}
