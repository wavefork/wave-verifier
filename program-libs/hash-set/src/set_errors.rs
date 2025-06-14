use thiserror::Error;
use solana_program::program_error::ProgramError;

#[derive(Error, Debug, Copy, Clone)]
pub enum HashSetError {
    #[error("Item Not Found")]
    ItemNotFound,
    #[error("Set is Full")]
    SetFull,
    #[error("Operation Not Allowed")]
    OperationNotAllowed,
}

impl From<HashSetError> for ProgramError {
    fn from(e: HashSetError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion() {
        let error: ProgramError = HashSetError::ItemNotFound.into();
        assert_eq!(error, ProgramError::Custom(HashSetError::ItemNotFound as u32));
    }
} 