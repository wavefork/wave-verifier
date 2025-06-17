/// Seeds for PDA derivation
pub const NULLIFIER_SEED: &[u8] = b"nullifier";
pub const REGISTRY_SEED: &[u8] = b"registry";
pub const PROOF_LOG_SEED: &[u8] = b"proof_log";

/// Size limits
pub const MAX_PROOF_SIZE: usize = 1024;
pub const MAX_PUBLIC_INPUTS_SIZE: usize = 256;
pub const MAX_FLOW_ID: u64 = 1000000;

/// Flow tags
pub const FLOW_TAG_MERKLE: u8 = 1;
pub const FLOW_TAG_DIRECT: u8 = 2;

// Program version
pub const PROGRAM_VERSION: u8 = 1;

// Test data for verification
#[cfg(test)]
pub mod test_data {
    // Flow IDs
    pub const FLOW_ID_1: u64 = 1;
    pub const FLOW_ID_2: u64 = 2;
    pub const FLOW_ID_3: u64 = 3;

    // Circuit hashes
    pub const CIRCUIT_HASH_1: [u8; 32] = [1u8; 32];
    pub const CIRCUIT_HASH_2: [u8; 32] = [2u8; 32];
    pub const CIRCUIT_HASH_3: [u8; 32] = [3u8; 32];

    // Merkle roots
    pub const MERKLE_ROOT_1: [u8; 32] = [10u8; 32];
    pub const MERKLE_ROOT_2: [u8; 32] = [20u8; 32];
    pub const MERKLE_ROOT_3: [u8; 32] = [30u8; 32];

    // Nullifiers
    pub const NULLIFIER_1: [u8; 32] = [40u8; 32];
    pub const NULLIFIER_2: [u8; 32] = [50u8; 32];
    pub const NULLIFIER_3: [u8; 32] = [60u8; 32];

    // Timestamps
    pub const TIMESTAMP_1: i64 = 1000000;
    pub const TIMESTAMP_2: i64 = 2000000;
    pub const TIMESTAMP_3: i64 = 3000000;

    // Proofs
    pub const PROOF_1: [u8; 128] = [70u8; 128];
    pub const PROOF_2: [u8; 128] = [80u8; 128];
    pub const PROOF_3: [u8; 128] = [90u8; 128];

    // Public inputs
    pub const PUBLIC_INPUTS_1: [u8; 32] = [100u8; 32];
    pub const PUBLIC_INPUTS_2: [u8; 32] = [110u8; 32];
    pub const PUBLIC_INPUTS_3: [u8; 32] = [120u8; 32];
}

// Account sizes
pub const FLOW_REGISTRY_SIZE: usize = 1024;
pub const NULLIFIER_SIZE: usize = 128;
pub const PROOF_LOG_SIZE: usize = 256;

// Program seeds
pub const FLOW_REGISTRY_SEED: &[u8] = b"flow_registry";
pub const NULLIFIER_SEED: &[u8] = b"nullifier";
pub const PROOF_LOG_SEED: &[u8] = b"proof_log";

// Verification parameters
pub const MAX_MERKLE_TREE_DEPTH: usize = 32;
pub const MAX_PUBLIC_INPUTS: usize = 10;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_uniqueness() {
        // Test that test constants have different values
        assert_ne!(test_data::FLOW_ID_1, test_data::FLOW_ID_2);
        assert_ne!(test_data::CIRCUIT_HASH_1, test_data::CIRCUIT_HASH_2);
        assert_ne!(test_data::MERKLE_ROOT_1, test_data::MERKLE_ROOT_2);
        assert_ne!(test_data::NULLIFIER_1, test_data::NULLIFIER_2);
        assert_ne!(test_data::TIMESTAMP_1, test_data::TIMESTAMP_2);
        assert_ne!(test_data::PROOF_1, test_data::PROOF_2);
        assert_ne!(test_data::PUBLIC_INPUTS_1, test_data::PUBLIC_INPUTS_2);
    }

    #[test]
    fn test_account_sizes() {
        assert!(FLOW_REGISTRY_SIZE >= 1024);
        assert!(NULLIFIER_SIZE >= 128);
        assert!(PROOF_LOG_SIZE >= 256);
    }
} 