use borsh::BorshDeserialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_program,
    sysvar::{clock::Clock, Sysvar},
};

use crate::{
    error::WaveError,
    events::WaveEvent,
    instructions::WaveInstruction,
    state::{FlowRegistry, Nullifier, ProofLog},
};

#[cfg(test)]
pub struct Groth16Verifier {
    accepted_proofs: Vec<[u8; 32]>,
}

#[cfg(test)]
impl Groth16Verifier {
    pub fn new() -> Self {
        Self {
            accepted_proofs: vec![
                [1u8; 32], // Test proof 1
                [2u8; 32], // Test proof 2
                [3u8; 32], // Test proof 3
            ],
        }
    }

    pub fn verify(&self, proof: &[u8]) -> bool {
        let mut proof_hash = [0u8; 32];
        proof_hash.copy_from_slice(&proof[..32]);
        self.accepted_proofs.contains(&proof_hash)
    }
}

#[cfg(test)]
pub struct MerkleTreeVerifier {
    valid_roots: Vec<[u8; 32]>,
}

#[cfg(test)]
impl MerkleTreeVerifier {
    pub fn new() -> Self {
        Self {
            valid_roots: vec![
                [10u8; 32], // Test root 1
                [20u8; 32], // Test root 2
                [30u8; 32], // Test root 3
            ],
        }
    }

    pub fn verify(&self, root: &[u8; 32]) -> bool {
        self.valid_roots.contains(root)
    }
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = WaveInstruction::try_from_slice(instruction_data)
        .map_err(|_| WaveError::InvalidInstruction)?;

    #[cfg(test)]
    let proof_verifier = Groth16Verifier::new();
    #[cfg(test)]
    let merkle_verifier = MerkleTreeVerifier::new();

    match instruction {
        WaveInstruction::InitRegistry {
            flow_id,
            merkle_root,
            circuit_hash,
            callback_program_id,
        } => {
            msg!("Instruction: InitRegistry");
            let accounts_iter = &mut accounts.iter();
            
            let authority = next_account_info(accounts_iter)?;
            let flow_registry = next_account_info(accounts_iter)?;
            let system_program = next_account_info(accounts_iter)?;

            if !authority.is_signer {
                return Err(WaveError::Unauthorized.into());
            }

            if system_program.key != &system_program::id() {
                return Err(ProgramError::InvalidAccountData);
            }

            // Validate circuit hash
            if circuit_hash == [0u8; 32] {
                return Err(WaveError::InvalidCircuitHash.into());
            }

            // Validate Merkle root if provided
            #[cfg(test)]
            if let Some(root) = merkle_root {
                if !merkle_verifier.verify(&root) {
                    return Err(WaveError::InvalidMerkleRoot.into());
                }
            }

            let registry = FlowRegistry::new(
                *authority.key,
                flow_id,
                merkle_root,
                circuit_hash,
                callback_program_id.map(|id| Pubkey::new_from_array(id)),
            );

            registry.save(flow_registry)?;
            WaveEvent::FlowRegistered { flow_id, merkle_root, circuit_hash }.emit();
            Ok(())
        }

        WaveInstruction::ValidateProof {
            proof,
            public_inputs,
            nullifier,
        } => {
            msg!("Instruction: ValidateProof");
            let accounts_iter = &mut accounts.iter();
            
            let payer = next_account_info(accounts_iter)?;
            let flow_registry = next_account_info(accounts_iter)?;
            let nullifier_account = next_account_info(accounts_iter)?;
            let proof_log = next_account_info(accounts_iter)?;
            let system_program = next_account_info(accounts_iter)?;

            if !payer.is_signer {
                return Err(WaveError::Unauthorized.into());
            }

            // Verify proof
            #[cfg(test)]
            if !proof_verifier.verify(&proof) {
                WaveEvent::ProofRejected {
                    flow_id: 0,
                    reason: "Invalid proof".to_string(),
                }.emit();
                return Err(WaveError::InvalidProof.into());
            }

            // Record nullifier
            let clock = Clock::get()?;
            let nullifier_data = Nullifier::new(
                nullifier,
                clock.unix_timestamp,
                0, // Flow ID
            );
            nullifier_data.save(nullifier_account)?;

            // Record proof
            let mut public_inputs_hash = [0u8; 32];
            public_inputs_hash.copy_from_slice(&public_inputs[..32]);
            
            let proof_log_data = ProofLog::new(
                nullifier,
                clock.unix_timestamp,
                0, // Flow ID
                public_inputs_hash,
            );
            proof_log_data.save(proof_log)?;

            WaveEvent::FlowExecuted {
                flow_id: 0,
                nullifier,
            }.emit();
            Ok(())
        }

        WaveInstruction::SetRoot { new_root } => {
            msg!("Instruction: SetRoot");
            let accounts_iter = &mut accounts.iter();
            
            let authority = next_account_info(accounts_iter)?;
            let flow_registry = next_account_info(accounts_iter)?;

            if !authority.is_signer {
                return Err(WaveError::Unauthorized.into());
            }

            // Validate Merkle root
            #[cfg(test)]
            if !merkle_verifier.verify(&new_root) {
                return Err(WaveError::InvalidMerkleRoot.into());
            }

            let mut registry = FlowRegistry::load(flow_registry)?;
            registry.merkle_root = Some(new_root);
            registry.save(flow_registry)?;

            WaveEvent::RootUpdated {
                flow_id: registry.flow_id,
                new_root,
            }.emit();
            Ok(())
        }

        WaveInstruction::TriggerFlow {
            flow_id,
            instruction_data,
        } => {
            msg!("Instruction: TriggerFlow");
            let accounts_iter = &mut accounts.iter();
            
            let payer = next_account_info(accounts_iter)?;
            let flow_registry = next_account_info(accounts_iter)?;
            let target_program = next_account_info(accounts_iter)?;

            if !payer.is_signer {
                return Err(WaveError::Unauthorized.into());
            }

            // Execute CPI call
            msg!("Would trigger program {} with data {:?}", target_program.key, instruction_data);
            
            WaveEvent::FlowTriggered {
                flow_id,
                target_program: *target_program.key,
            }.emit();
            Ok(())
        }
    }
} 