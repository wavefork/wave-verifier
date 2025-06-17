use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
};

pub mod init_registry;
pub mod set_root;
pub mod trigger_flow;
pub mod validate_proof;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum WaveInstruction {
    /// Initialize a new flow registry
    /// 
    /// Accounts expected:
    /// 0. `[signer]` The authority that will control this flow
    /// 1. `[writable]` The flow registry account to initialize
    /// 2. `[]` System program
    InitRegistry {
        flow_id: u64,
        merkle_root: Option<[u8; 32]>,
        circuit_hash: [u8; 32],
        callback_program_id: Option<[u8; 32]>,
    },

    /// Update the Merkle root for a flow
    /// 
    /// Accounts expected:
    /// 0. `[signer]` The flow authority
    /// 1. `[writable]` The flow registry account
    SetRoot {
        new_root: [u8; 32],
    },

    /// Validate a zero-knowledge proof
    /// 
    /// Accounts expected:
    /// 0. `[signer]` The fee payer
    /// 1. `[]` The flow registry account
    /// 2. `[writable]` The nullifier PDA
    /// 3. `[writable]` The proof log PDA (optional)
    /// 4. `[]` System program
    ValidateProof {
        proof: Vec<u8>,
        public_inputs: Vec<u8>,
        nullifier: [u8; 32],
    },

    /// Trigger downstream program after proof validation
    /// 
    /// Accounts expected by base instruction:
    /// 0. `[signer]` The fee payer
    /// 1. `[]` The flow registry account
    /// 2. `[]` The target program to call
    /// Additional accounts based on target program
    TriggerFlow {
        flow_id: u64,
        instruction_data: Vec<u8>,
    },
}

#[cfg(test)]
pub struct InstructionProcessor {
    pub last_instruction: Option<WaveInstruction>,
    pub instruction_count: usize,
    pub success: bool,
}

#[cfg(test)]
impl InstructionProcessor {
    pub fn new() -> Self {
        Self {
            last_instruction: None,
            instruction_count: 0,
            success: true,
        }
    }

    pub fn process_instruction(
        &mut self,
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> Result<(), ProgramError> {
        let instruction = WaveInstruction::try_from_slice(instruction_data)?;
        self.last_instruction = Some(instruction);
        self.instruction_count += 1;
        
        if self.success {
            Ok(())
        } else {
            Err(ProgramError::Custom(0))
        }
    }

    pub fn clear(&mut self) {
        self.last_instruction = None;
        self.instruction_count = 0;
        self.success = true;
    }

    pub fn set_success(&mut self, success: bool) {
        self.success = success;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::test_data::*;

    #[test]
    fn test_instruction_processing() {
        let mut processor = InstructionProcessor::new();
        
        let instruction = WaveInstruction::InitRegistry {
            flow_id: FLOW_ID_1,
            merkle_root: Some(MERKLE_ROOT_1),
            circuit_hash: CIRCUIT_HASH_1,
            callback_program_id: None,
        };
        
        let instruction_data = instruction.try_to_vec().unwrap();
        let program_id = Pubkey::new_unique();
        let accounts = vec![];
        
        assert!(processor.process_instruction(&program_id, &accounts, &instruction_data).is_ok());
        assert_eq!(processor.instruction_count, 1);
        
        processor.set_success(false);
        assert!(processor.process_instruction(&program_id, &accounts, &instruction_data).is_err());
        
        processor.clear();
        assert_eq!(processor.instruction_count, 0);
        assert!(processor.success);
    }

    #[test]
    fn test_instruction_serialization() {
        let instructions = vec![
            WaveInstruction::InitRegistry {
                flow_id: FLOW_ID_1,
                merkle_root: Some(MERKLE_ROOT_1),
                circuit_hash: CIRCUIT_HASH_1,
                callback_program_id: None,
            },
            WaveInstruction::SetRoot {
                new_root: MERKLE_ROOT_2,
            },
            WaveInstruction::ValidateProof {
                proof: PROOF_1.to_vec(),
                public_inputs: PUBLIC_INPUTS_1.to_vec(),
                nullifier: NULLIFIER_1,
            },
            WaveInstruction::TriggerFlow {
                flow_id: FLOW_ID_2,
                instruction_data: vec![1, 2, 3],
            },
        ];

        for instruction in instructions {
            let serialized = instruction.try_to_vec().unwrap();
            let deserialized = WaveInstruction::try_from_slice(&serialized).unwrap();
            
            match (instruction, deserialized) {
                (
                    WaveInstruction::InitRegistry { flow_id: f1, merkle_root: m1, circuit_hash: c1, callback_program_id: p1 },
                    WaveInstruction::InitRegistry { flow_id: f2, merkle_root: m2, circuit_hash: c2, callback_program_id: p2 }
                ) => {
                    assert_eq!(f1, f2);
                    assert_eq!(m1, m2);
                    assert_eq!(c1, c2);
                    assert_eq!(p1, p2);
                }
                (
                    WaveInstruction::SetRoot { new_root: r1 },
                    WaveInstruction::SetRoot { new_root: r2 }
                ) => {
                    assert_eq!(r1, r2);
                }
                (
                    WaveInstruction::ValidateProof { proof: p1, public_inputs: i1, nullifier: n1 },
                    WaveInstruction::ValidateProof { proof: p2, public_inputs: i2, nullifier: n2 }
                ) => {
                    assert_eq!(p1, p2);
                    assert_eq!(i1, i2);
                    assert_eq!(n1, n2);
                }
                (
                    WaveInstruction::TriggerFlow { flow_id: f1, instruction_data: d1 },
                    WaveInstruction::TriggerFlow { flow_id: f2, instruction_data: d2 }
                ) => {
                    assert_eq!(f1, f2);
                    assert_eq!(d1, d2);
                }
                _ => panic!("Instructions don't match after serialization/deserialization"),
            }
        }
    }
} 