use thiserror::Error;
use solana_program::program_error::ProgramError;

#[derive(Error, Debug, Copy, Clone)]
pub enum CompressionError {
    #[error("Compression Failed")]
    CompressionFailed,
    #[error("Decompression Failed")]
    DecompressionFailed,
    #[error("Invalid Compression Type")]
    InvalidCompressionType,
}

impl From<CompressionError> for ProgramError {
    fn from(e: CompressionError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion() {
        let error: ProgramError = CompressionError::CompressionFailed.into();
        assert_eq!(error, ProgramError::Custom(CompressionError::CompressionFailed as u32));
    }
} 