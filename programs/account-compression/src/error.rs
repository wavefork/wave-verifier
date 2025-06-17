use solana_program::{program_error::ProgramError, decode_error::DecodeError};
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone, PartialEq)]
pub enum CompressionError {
    #[error("Invalid compression algorithm")]
    InvalidAlgorithm,
    
    #[error("Compression failed")]
    CompressionFailed,
    
    #[error("Decompression failed")]
    DecompressionFailed,
    
    #[error("Invalid account state")]
    InvalidAccountState,
    
    #[error("Buffer overflow")]
    BufferOverflow,
    
    #[error("Invalid compression level")]
    InvalidCompressionLevel,
    
    #[error("Account already compressed")]
    AlreadyCompressed,
    
    #[error("Account not compressed")]
    NotCompressed,
    
    #[error("Invalid chunk size")]
    InvalidChunkSize,
    
    #[error("Hash mismatch")]
    HashMismatch,
    
    #[error("Insufficient buffer size")]
    InsufficientBufferSize,
    
    #[error("Invalid account type")]
    InvalidAccountType,
    
    #[error("Unauthorized operation")]
    Unauthorized,
}

impl From<CompressionError> for ProgramError {
    fn from(e: CompressionError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for CompressionError {
    fn type_of() -> &'static str {
        "CompressionError"
    }
} 