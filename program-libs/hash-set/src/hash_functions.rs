use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub fn calculate_hash<T: Hash>(item: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    item.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_calculation() {
        let item = "test_item";
        let hash1 = calculate_hash(&item);
        let hash2 = calculate_hash(&item);
        assert_eq!(hash1, hash2);
    }
} 