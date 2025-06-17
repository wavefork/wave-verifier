use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    program_error::ProgramError,
};

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq)]
pub struct Nullifier {
    /// The nullifier hash
    pub hash: [u8; 32],
    /// When this nullifier was used
    pub timestamp: i64,
    /// The flow ID this nullifier was used with
    pub flow_id: u64,
}

impl Nullifier {
    pub const SIZE: usize = 32 + 8 + 8;

    pub fn new(hash: [u8; 32], timestamp: i64, flow_id: u64) -> Self {
        Self {
            hash,
            timestamp,
            flow_id,
        }
    }

    pub fn save(&self, account: &AccountInfo) -> Result<(), ProgramError> {
        let data = self.try_to_vec()?;
        let mut account_data = account.try_borrow_mut_data()?;
        account_data[..data.len()].copy_from_slice(&data);
        Ok(())
    }

    pub fn load(account: &AccountInfo) -> Result<Self, ProgramError> {
        let data = account.try_borrow_data()?;
        let nullifier = Self::try_from_slice(&data)?;
        Ok(nullifier)
    }
}

#[cfg(test)]
pub struct NullifierSet {
    pub nullifiers: Vec<Nullifier>,
}

#[cfg(test)]
impl NullifierSet {
    pub fn new() -> Self {
        Self {
            nullifiers: Vec::new(),
        }
    }

    pub fn add(&mut self, nullifier: Nullifier) {
        self.nullifiers.push(nullifier);
    }

    pub fn exists(&self, hash: &[u8; 32]) -> bool {
        self.nullifiers.iter().any(|n| n.hash == *hash)
    }

    pub fn get(&self, hash: &[u8; 32]) -> Option<&Nullifier> {
        self.nullifiers.iter().find(|n| n.hash == *hash)
    }

    pub fn get_by_flow(&self, flow_id: u64) -> Vec<&Nullifier> {
        self.nullifiers.iter().filter(|n| n.flow_id == flow_id).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::test_data::*;

    #[test]
    fn test_nullifier() {
        let nullifier = Nullifier::new(
            NULLIFIER_1,
            TIMESTAMP_1,
            FLOW_ID_1,
        );

        assert_eq!(nullifier.hash, NULLIFIER_1);
        assert_eq!(nullifier.timestamp, TIMESTAMP_1);
        assert_eq!(nullifier.flow_id, FLOW_ID_1);
    }

    #[test]
    fn test_nullifier_set() {
        let mut set = NullifierSet::new();
        
        let nullifier1 = Nullifier::new(
            NULLIFIER_1,
            TIMESTAMP_1,
            FLOW_ID_1,
        );
        set.add(nullifier1);

        let nullifier2 = Nullifier::new(
            NULLIFIER_2,
            TIMESTAMP_2,
            FLOW_ID_1,
        );
        set.add(nullifier2);

        let nullifier3 = Nullifier::new(
            NULLIFIER_3,
            TIMESTAMP_3,
            FLOW_ID_2,
        );
        set.add(nullifier3);

        assert!(set.exists(&NULLIFIER_1));
        assert!(!set.exists(&[0u8; 32]));

        let found = set.get(&NULLIFIER_1).unwrap();
        assert_eq!(found.flow_id, FLOW_ID_1);

        let flow1_nullifiers = set.get_by_flow(FLOW_ID_1);
        assert_eq!(flow1_nullifiers.len(), 2);
        assert_eq!(flow1_nullifiers[0].flow_id, FLOW_ID_1);
        assert_eq!(flow1_nullifiers[1].flow_id, FLOW_ID_1);

        let flow2_nullifiers = set.get_by_flow(FLOW_ID_2);
        assert_eq!(flow2_nullifiers.len(), 1);
        assert_eq!(flow2_nullifiers[0].flow_id, FLOW_ID_2);
    }
} 