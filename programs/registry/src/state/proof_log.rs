use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    program_error::ProgramError,
};

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq)]
pub struct ProofLog {
    /// The nullifier hash
    pub nullifier: [u8; 32],
    /// When this proof was submitted
    pub timestamp: i64,
    /// The flow ID this proof was used with
    pub flow_id: u64,
    /// Public inputs hash
    pub public_inputs_hash: [u8; 32],
}

impl ProofLog {
    pub const SIZE: usize = 32 + 8 + 8 + 32;

    pub fn new(
        nullifier: [u8; 32],
        timestamp: i64,
        flow_id: u64,
        public_inputs_hash: [u8; 32],
    ) -> Self {
        Self {
            nullifier,
            timestamp,
            flow_id,
            public_inputs_hash,
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
        let log = Self::try_from_slice(&data)?;
        Ok(log)
    }
}

#[cfg(test)]
pub struct ProofHistory {
    pub logs: Vec<ProofLog>,
}

#[cfg(test)]
impl ProofHistory {
    pub fn new() -> Self {
        Self {
            logs: Vec::new(),
        }
    }

    pub fn add_log(&mut self, log: ProofLog) {
        self.logs.push(log);
    }

    pub fn get_by_flow(&self, flow_id: u64) -> Vec<&ProofLog> {
        self.logs.iter().filter(|l| l.flow_id == flow_id).collect()
    }

    pub fn get_by_nullifier(&self, nullifier: &[u8; 32]) -> Vec<&ProofLog> {
        self.logs.iter().filter(|l| l.nullifier == *nullifier).collect()
    }

    pub fn get_by_timerange(&self, start: i64, end: i64) -> Vec<&ProofLog> {
        self.logs.iter()
            .filter(|l| l.timestamp >= start && l.timestamp <= end)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::test_data::*;

    #[test]
    fn test_proof_log() {
        let log = ProofLog::new(
            NULLIFIER_1,
            TIMESTAMP_1,
            FLOW_ID_1,
            PUBLIC_INPUTS_1,
        );

        assert_eq!(log.nullifier, NULLIFIER_1);
        assert_eq!(log.timestamp, TIMESTAMP_1);
        assert_eq!(log.flow_id, FLOW_ID_1);
        assert_eq!(log.public_inputs_hash, PUBLIC_INPUTS_1);
    }

    #[test]
    fn test_proof_history() {
        let mut history = ProofHistory::new();
        
        let log1 = ProofLog::new(
            NULLIFIER_1,
            TIMESTAMP_1,
            FLOW_ID_1,
            PUBLIC_INPUTS_1,
        );
        history.add_log(log1);

        let log2 = ProofLog::new(
            NULLIFIER_2,
            TIMESTAMP_2,
            FLOW_ID_1,
            PUBLIC_INPUTS_2,
        );
        history.add_log(log2);

        let log3 = ProofLog::new(
            NULLIFIER_3,
            TIMESTAMP_3,
            FLOW_ID_2,
            PUBLIC_INPUTS_3,
        );
        history.add_log(log3);

        let flow1_logs = history.get_by_flow(FLOW_ID_1);
        assert_eq!(flow1_logs.len(), 2);
        assert_eq!(flow1_logs[0].flow_id, FLOW_ID_1);
        assert_eq!(flow1_logs[1].flow_id, FLOW_ID_1);

        let nullifier1_logs = history.get_by_nullifier(&NULLIFIER_1);
        assert_eq!(nullifier1_logs.len(), 1);
        assert_eq!(nullifier1_logs[0].nullifier, NULLIFIER_1);

        let timerange_logs = history.get_by_timerange(
            TIMESTAMP_1,
            TIMESTAMP_2,
        );
        assert_eq!(timerange_logs.len(), 2);
        assert!(timerange_logs.iter().all(|l| l.timestamp >= TIMESTAMP_1
            && l.timestamp <= TIMESTAMP_2));
    }
} 