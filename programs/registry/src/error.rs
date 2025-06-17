use num_derive::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::ProgramError,
};
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone)]
pub enum WaveError {
    #[error("Invalid instruction")]
    InvalidInstruction,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Invalid flow ID")]
    InvalidFlowId,

    #[error("Invalid circuit hash")]
    InvalidCircuitHash,

    #[error("Invalid Merkle root")]
    InvalidMerkleRoot,

    #[error("Invalid proof")]
    InvalidProof,

    #[error("Invalid nullifier")]
    InvalidNullifier,

    #[error("Nullifier already used")]
    NullifierAlreadyUsed,

    #[error("Flow disabled")]
    FlowDisabled,

    #[error("Invalid callback program")]
    InvalidCallbackProgram,

    #[error("Invalid account data")]
    InvalidAccountData,
}

impl From<WaveError> for ProgramError {
    fn from(e: WaveError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for WaveError {
    fn type_of() -> &'static str {
        "WaveError"
    }
}

#[cfg(test)]
pub struct TestErrorHandler {
    pub last_error: Option<WaveError>,
    pub error_count: usize,
}

#[cfg(test)]
impl TestErrorHandler {
    pub fn new() -> Self {
        Self {
            last_error: None,
            error_count: 0,
        }
    }

    pub fn handle_error(&mut self, error: WaveError) -> ProgramError {
        self.last_error = Some(error);
        self.error_count += 1;
        error.into()
    }

    pub fn clear(&mut self) {
        self.last_error = None;
        self.error_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_handler() {
        let mut handler = TestErrorHandler::new();
        
        let error = handler.handle_error(WaveError::InvalidInstruction);
        assert!(matches!(error, ProgramError::Custom(_)));
        assert_eq!(handler.error_count, 1);
        assert!(matches!(handler.last_error, Some(WaveError::InvalidInstruction)));
        
        handler.clear();
        assert_eq!(handler.error_count, 0);
        assert!(handler.last_error.is_none());
    }

    #[test]
    fn test_error_conversion() {
        let errors = vec![
            WaveError::InvalidInstruction,
            WaveError::Unauthorized,
            WaveError::InvalidFlowId,
            WaveError::InvalidCircuitHash,
            WaveError::InvalidMerkleRoot,
            WaveError::InvalidProof,
            WaveError::InvalidNullifier,
            WaveError::NullifierAlreadyUsed,
            WaveError::FlowDisabled,
            WaveError::InvalidCallbackProgram,
            WaveError::InvalidAccountData,
        ];

        for error in errors {
            let program_error: ProgramError = error.into();
            assert!(matches!(program_error, ProgramError::Custom(_)));
            
            let error_message = format!("{}", error);
            assert!(!error_message.is_empty());
        }
    }
} 