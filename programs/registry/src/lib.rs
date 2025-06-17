use solana_program::{
    account_info::AccountInfo,
    entrypoint,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

pub mod constants;
pub mod error;
pub mod events;
pub mod instructions;
pub mod processor;
pub mod state;

use processor::process_instruction;

entrypoint!(process_instruction);

#[cfg(test)]
pub mod test_utils {
    use super::*;
    use crate::{
        state::{
            flow_registry::RegistryManager,
            nullifier::NullifierSet,
            proof_log::ProofHistory,
        },
        instructions::WaveInstruction,
    };

    pub struct TestEnvironment {
        pub registry_manager: RegistryManager,
        pub nullifier_set: NullifierSet,
        pub proof_history: ProofHistory,
    }

    impl TestEnvironment {
        pub fn new() -> Self {
            Self {
                registry_manager: RegistryManager::new(),
                nullifier_set: NullifierSet::new(),
                proof_history: ProofHistory::new(),
            }
        }

        pub fn process_instruction(
            &mut self,
            program_id: &Pubkey,
            accounts: &[AccountInfo],
            instruction_data: &[u8],
        ) -> ProgramResult {
            let instruction = WaveInstruction::try_from_slice(instruction_data)?;
            
            match instruction {
                WaveInstruction::InitRegistry { 
                    flow_id, 
                    merkle_root, 
                    circuit_hash, 
                    callback_program_id 
                } => {
                    let registry = state::flow_registry::FlowRegistry::new(
                        *accounts[0].key,
                        flow_id,
                        merkle_root,
                        circuit_hash,
                        callback_program_id.map(|id| Pubkey::new_from_array(id)),
                    );
                    self.registry_manager.register(registry);
                    Ok(())
                }
                WaveInstruction::SetRoot { new_root } => {
                    let registry = self.registry_manager.get_by_id(0).ok_or(error::WaveError::FlowNotRegistered)?;
                    if accounts[0].key != &registry.authority {
                        return Err(error::WaveError::Unauthorized.into());
                    }
                    self.registry_manager.update_root(0, new_root)?;
                    Ok(())
                }
                WaveInstruction::ValidateProof { 
                    proof, 
                    public_inputs, 
                    nullifier 
                } => {
                    if self.nullifier_set.exists(&nullifier) {
                        return Err(error::WaveError::NullifierAlreadyUsed.into());
                    }
                    
                    let timestamp = 0i64; // For testing
                    let flow_id = 0u64; // For testing
                    
                    let nullifier_entry = state::nullifier::Nullifier::new(
                        nullifier,
                        timestamp,
                        flow_id,
                    );
                    self.nullifier_set.add(nullifier_entry);
                    
                    let mut public_inputs_hash = [0u8; 32];
                    public_inputs_hash.copy_from_slice(&public_inputs[..32]);
                    
                    let proof_log = state::proof_log::ProofLog::new(
                        nullifier,
                        timestamp,
                        flow_id,
                        public_inputs_hash,
                    );
                    self.proof_history.record(proof_log);
                    
                    Ok(())
                }
                WaveInstruction::TriggerFlow { 
                    flow_id, 
                    instruction_data 
                } => {
                    let registry = self.registry_manager.get_by_id(flow_id)
                        .ok_or(error::WaveError::FlowNotRegistered)?;
                    
                    if !registry.is_enabled {
                        return Err(error::WaveError::InvalidInstruction.into());
                    }
                    
                    // In test environment, just verify the accounts are present
                    if accounts.len() < 3 {
                        return Err(error::WaveError::InvalidInstruction.into());
                    }
                    
                    Ok(())
                }
            }
        }

        pub fn reset(&mut self) {
            self.registry_manager.reset();
            self.nullifier_set.reset();
            self.proof_history.reset();
        }
    }
} 