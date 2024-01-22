use uuid::Uuid;

pub struct MurMurHasher;

impl MurMurHasher {
    pub fn hash_str(str: &str) -> String {
        MurMurHasher::hash_bytes(str.as_bytes())
    }

    pub fn hash_bytes(bytes: &[u8]) -> String {
        let hash = fastmurmur3::murmur3_x64_128(bytes, 0);
        let guid = Uuid::from_bytes(hash.to_le_bytes());
        let result = guid.to_string().to_lowercase().replace("-", "");
        result
    }
}
