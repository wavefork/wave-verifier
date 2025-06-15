use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        program_error::ProgramError,
        pubkey::Pubkey,
        clock::UnixTimestamp,
    },
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct Batch {
    pub id: u64,
    pub items: Vec<[u8; 32]>,
    pub timestamp: UnixTimestamp,
    pub processor: Pubkey,
    pub status: BatchStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum BatchStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

impl Batch {
    pub fn new(id: u64, items: Vec<[u8; 32]>, processor: Pubkey) -> Self {
        Self {
            id,
            items,
            timestamp: 0, // Should be set from blockchain
            processor,
            status: BatchStatus::Pending,
        }
    }

    pub fn process(&mut self) -> Result<(), ProgramError> {
        if self.status != BatchStatus::Pending {
            return Err(ProgramError::InvalidArgument);
        }

        self.status = BatchStatus::Processing;
        // Simulate processing
        self.status = BatchStatus::Completed;
        Ok(())
    }

    pub fn fail(&mut self) {
        self.status = BatchStatus::Failed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_creation() {
        let processor = Pubkey::new_unique();
        let items = vec![[1u8; 32], [2u8; 32]];
        let batch = Batch::new(1, items.clone(), processor);

        assert_eq!(batch.id, 1);
        assert_eq!(batch.items, items);
        assert_eq!(batch.status, BatchStatus::Pending);
    }

    #[test]
    fn test_batch_processing() {
        let processor = Pubkey::new_unique();
        let items = vec![[1u8; 32], [2u8; 32]];
        let mut batch = Batch::new(1, items, processor);

        assert!(batch.process().is_ok());
        assert_eq!(batch.status, BatchStatus::Completed);
    }

    #[test]
    fn test_batch_failure() {
        let processor = Pubkey::new_unique();
        let items = vec![[1u8; 32], [2u8; 32]];
        let mut batch = Batch::new(1, items, processor);

        batch.fail();
        assert_eq!(batch.status, BatchStatus::Failed);
    }
} 