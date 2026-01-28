use sha2::{Sha256, Digest};

pub fn generate_id(name: &str) -> String {

    // Create hash from name only (matching Go server behavior)
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    let hash = hasher.finalize();

    // Take first 16 bytes (128 bits) and convert to big integer
    let mut num = [0u8; 16];
    num.copy_from_slice(&hash[..16]);

    // Convert to u128 and right shift by 9 bits to get 119 bits
    let mut value = u128::from_be_bytes(num);
    value >>= 9;

    // Convert to base62 (20 characters)
    let mut id = String::with_capacity(20);
    for _ in 0..20 {
        let remainder = (value % 62) as u8;
        value /= 62;

        let c = if remainder < 10 {
            (remainder + 48) as char  // 0-9
        } else if remainder < 36 {
            (remainder + 65 - 10) as char  // A-Z
        } else {
            (remainder + 97 - 36) as char  // a-z
        };
        id.push(c);
    }

    id
}
