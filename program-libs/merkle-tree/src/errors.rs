use thiserror::Error;
use solana_program::program_error::ProgramError;

#[derive(Error, Debug, Copy, Clone)]
pub enum MerkleTreeError {
    #[error("Invalid Merkle Tree Depth")]
    InvalidDepth,
    #[error("Tree is Full")]
    TreeFull,
    #[error("Invalid Proof")]
    InvalidProof,
    #[error("Batch Processing Error")]
    BatchProcessingError,
}

impl From<MerkleTreeError> for ProgramError {
    fn from(e: MerkleTreeError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion() {
        let error: ProgramError = MerkleTreeError::InvalidDepth.into();
        assert_eq!(error, ProgramError::Custom(MerkleTreeError::InvalidDepth as u32));
    }
} 