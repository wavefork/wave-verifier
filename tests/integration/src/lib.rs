use {
    anyhow::Result,
    solana_program_test::*,
    solana_sdk::{
        account::Account,
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        transaction::Transaction,
    },
    wave_verifier::{
        instruction::CloudVerifierInstruction,
        state::{FlowRegistry, Nullifier, ProofLog},
    },
    wave_verifier_sdk::{WaveClient, types::{Flow, Proof}},
};

use solana_program::{
    account_info::AccountInfo,
    program_error::ProgramError,
    system_program,
    clock::Clock,
    sysvar::Sysvar,
};

use wave_verifier::{
    constants::test_data::*,
    instructions::WaveInstruction,
};

pub struct Proof {
    pub proof_bytes: Vec<u8>,
    pub public_inputs: Vec<u8>,
    pub nullifier: [u8; 32],
}

pub struct Flow {
    pub id: u64,
    pub merkle_root: Option<[u8; 32]>,
    pub circuit_hash: [u8; 32],
    pub callback_program_id: Option<[u8; 32]>,
}

mod common {
    use super::*;
    use solana_program_test::ProgramTest;

    pub async fn setup() -> (BanksClient, Keypair, Hash) {
        let program_id = Pubkey::new_unique();
        let mut program_test = ProgramTest::new(
            "wave_verifier",
            program_id,
            processor!(wave_verifier::processor::process_instruction),
        );

        let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
        
        // Fund the payer
        banks_client
            .process_transaction(Transaction::new_signed_with_payer(
                &[solana_sdk::system_instruction::transfer(
                    &payer.pubkey(),
                    &payer.pubkey(),
                    1_000_000_000,
                )],
                Some(&payer.pubkey()),
                &[&payer],
                recent_blockhash,
            ))
            .await
            .unwrap();

        (banks_client, payer, recent_blockhash)
    }

    pub fn create_test_proof() -> Proof {
        Proof {
            proof_bytes: PROOF_1.to_vec(),
            public_inputs: PUBLIC_INPUTS_1.to_vec(),
            nullifier: NULLIFIER_1,
        }
    }

    pub fn create_test_flow() -> Flow {
        Flow {
            id: FLOW_ID_1,
            merkle_root: Some(MERKLE_ROOT_1),
            circuit_hash: CIRCUIT_HASH_1,
            callback_program_id: None,
        }
    }
}

mod flow_tests;
mod proof_tests;
mod nullifier_tests;

mod compression_tests {
    use super::*;

    async fn setup_program_test() -> (BanksClient, Keypair, Hash) {
        let program_id = Pubkey::new_unique();
        let (mut banks_client, payer, recent_blockhash) = ProgramTest::new(
            "account_compression",
            program_id,
            processor!(account_compression::processor::process_instruction),
        )
        .start()
        .await;
        
        (banks_client, payer, recent_blockhash)
    }

    fn create_test_account(size: usize) -> (Keypair, Vec<u8>) {
        let account = Keypair::new();
        let data = vec![42u8; size]; // Fill with test data
        (account, data)
    }

    #[tokio::test]
    async fn test_initialize_compression() {
        let (mut banks_client, payer, recent_blockhash) = setup_program_test().await;
        
        // Create state account
        let state_account = Keypair::new();
        let rent = banks_client.get_rent().await.unwrap();
        let state_size = 1024;
        let lamports = rent.minimum_balance(state_size);
        
        let transaction = Transaction::new_signed_with_payer(
            &[
                system_instruction::create_account(
                    &payer.pubkey(),
                    &state_account.pubkey(),
                    lamports,
                    state_size as u64,
                    &program_id,
                ),
                account_compression::instruction::initialize_compression(
                    &program_id,
                    &state_account.pubkey(),
                    32,
                    1024,
                ),
            ],
            Some(&payer.pubkey()),
            &[&payer, &state_account],
            recent_blockhash,
        );
        
        banks_client.process_transaction(transaction).await.unwrap();
        
        // Verify state
        let state = banks_client
            .get_account(state_account.pubkey())
            .await
            .unwrap()
            .unwrap();
            
        let compression_state = CompressionState::unpack_from_slice(&state.data).unwrap();
        assert!(compression_state.is_initialized);
        assert_eq!(compression_state.max_depth, 32);
        assert_eq!(compression_state.max_buffer_size, 1024);
    }

    #[tokio::test]
    async fn test_compress_and_decompress_account() {
        let (mut banks_client, payer, recent_blockhash) = setup_program_test().await;
        
        // Create test account with sample data
        let (test_account, test_data) = create_test_account(1000);
        let config = GlobalCompressionConfig {
            default_algorithm: CompressionAlgorithm::Lz4,
            min_chunk_size: 512,
            max_chunk_size: 4096,
            concurrent_compressions_limit: 4,
            verify_all_compressions: true,
            auto_decompress_on_access: false,
        };
        
        // Compress account
        let transaction = Transaction::new_signed_with_payer(
            &[account_compression::instruction::compress_account(
                &program_id,
                &test_account.pubkey(),
                AccountType::User,
                config.clone(),
            )],
            Some(&payer.pubkey()),
            &[&payer],
            recent_blockhash,
        );
        
        banks_client.process_transaction(transaction).await.unwrap();
        
        // Verify compression
        let compressed_account = banks_client
            .get_account(test_account.pubkey())
            .await
            .unwrap()
            .unwrap();
        
        assert!(compressed_account.data.len() < test_data.len());
        
        // Decompress account
        let transaction = Transaction::new_signed_with_payer(
            &[account_compression::instruction::decompress_account(
                &program_id,
                &test_account.pubkey(),
            )],
            Some(&payer.pubkey()),
            &[&payer],
            recent_blockhash,
        );
        
        banks_client.process_transaction(transaction).await.unwrap();
        
        // Verify decompression
        let decompressed_account = banks_client
            .get_account(test_account.pubkey())
            .await
            .unwrap()
            .unwrap();
        
        assert_eq!(decompressed_account.data, test_data);
    }

    #[tokio::test]
    async fn test_compression_queue() {
        let (mut banks_client, payer, recent_blockhash) = setup_program_test().await;
        
        // Create multiple test accounts
        let accounts: Vec<(Keypair, Vec<u8>)> = (0..5)
            .map(|_| create_test_account(1000))
            .collect();
        
        // Add accounts to compression queue
        for (account, _) in &accounts {
            let transaction = Transaction::new_signed_with_payer(
                &[account_compression::instruction::enqueue_compression(
                    &program_id,
                    &account.pubkey(),
                )],
                Some(&payer.pubkey()),
                &[&payer],
                recent_blockhash,
            );
            
            banks_client.process_transaction(transaction).await.unwrap();
        }
        
        // Process queue
        let transaction = Transaction::new_signed_with_payer(
            &[account_compression::instruction::process_compression_queue(
                &program_id,
            )],
            Some(&payer.pubkey()),
            &[&payer],
            recent_blockhash,
        );
        
        banks_client.process_transaction(transaction).await.unwrap();
        
        // Verify all accounts are compressed
        for (account, original_data) in &accounts {
            let compressed_account = banks_client
                .get_account(account.pubkey())
                .await
                .unwrap()
                .unwrap();
                
            assert!(compressed_account.data.len() < original_data.len());
        }
    }

    #[tokio::test]
    async fn test_error_conditions() {
        let (mut banks_client, payer, recent_blockhash) = setup_program_test().await;
        
        // Test invalid compression algorithm
        let (test_account, _) = create_test_account(1000);
        let invalid_config = GlobalCompressionConfig {
            default_algorithm: CompressionAlgorithm::Zstd, // Assuming Zstd is not supported
            min_chunk_size: 512,
            max_chunk_size: 4096,
            concurrent_compressions_limit: 4,
            verify_all_compressions: true,
            auto_decompress_on_access: false,
        };
        
        let transaction = Transaction::new_signed_with_payer(
            &[account_compression::instruction::compress_account(
                &program_id,
                &test_account.pubkey(),
                AccountType::User,
                invalid_config,
            )],
            Some(&payer.pubkey()),
            &[&payer],
            recent_blockhash,
        );
        
        let result = banks_client.process_transaction(transaction).await;
        assert!(result.is_err());
        
        // Test decompression of non-compressed account
        let transaction = Transaction::new_signed_with_payer(
            &[account_compression::instruction::decompress_account(
                &program_id,
                &test_account.pubkey(),
            )],
            Some(&payer.pubkey()),
            &[&payer],
            recent_blockhash,
        );
        
        let result = banks_client.process_transaction(transaction).await;
        assert!(result.is_err());
    }
}

// Additional test modules can be added here for specific features
mod compression_queue_tests {
    use super::*;

    #[tokio::test]
    async fn test_queue_overflow() {
        // Test queue overflow conditions
    }

    #[tokio::test]
    async fn test_queue_concurrent_access() {
        // Test concurrent queue access
    }
}

mod compression_stats_tests {
    use super::*;

    #[tokio::test]
    async fn test_compression_stats_tracking() {
        // Test compression statistics tracking
    }

    #[tokio::test]
    async fn test_compression_ratio_calculation() {
        // Test compression ratio calculations
    }
}

#[tokio::test]
async fn test_flow_registration() -> Result<()> {
    let (mut banks_client, payer, recent_blockhash) = common::setup().await;
    let flow = common::create_test_flow();
    
    let flow_id = 1u64;
    let flow_registry_key = Pubkey::find_program_address(
        &[b"registry", &flow_id.to_le_bytes()],
        &wave_verifier::id(),
    ).0;

    let ix = CloudVerifierInstruction::InitRegistry {
        flow_id,
        merkle_root: Some(flow.merkle_root),
        circuit_hash: flow.circuit_hash,
        callback_program_id: None,
    };

    let transaction = Transaction::new_signed_with_payer(
        &[Instruction::new_with_borsh(
            wave_verifier::id(),
            &ix,
            vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(flow_registry_key, false),
                AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            ],
        )],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    banks_client.process_transaction(transaction).await?;

    let flow_registry = banks_client.get_account(flow_registry_key).await?.unwrap();
    let flow_data = FlowRegistry::try_from_slice(&flow_registry.data)?;
    assert_eq!(flow_data.flow_id, flow_id);
    assert_eq!(flow_data.merkle_root, Some(flow.merkle_root));
    assert_eq!(flow_data.circuit_hash, flow.circuit_hash);

    Ok(())
}

#[tokio::test]
async fn test_proof_verification() -> Result<()> {
    let (mut banks_client, payer, recent_blockhash) = common::setup().await;
    let flow = common::create_test_flow();
    let proof = common::create_test_proof();
    
    let flow_id = 1u64;
    let nullifier = [3u8; 32];
    
    let nullifier_key = Pubkey::find_program_address(
        &[b"nullifier", &nullifier],
        &wave_verifier::id(),
    ).0;

    let proof_log_key = Pubkey::find_program_address(
        &[b"proof_log", &nullifier],
        &wave_verifier::id(),
    ).0;

    let ix = CloudVerifierInstruction::ValidateProof {
        proof: proof.proof_bytes,
        public_inputs: proof.public_inputs,
        nullifier,
    };

    let transaction = Transaction::new_signed_with_payer(
        &[Instruction::new_with_borsh(
            wave_verifier::id(),
            &ix,
            vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(flow_registry_key, false),
                AccountMeta::new(nullifier_key, false),
                AccountMeta::new(proof_log_key, false),
                AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            ],
        )],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    banks_client.process_transaction(transaction).await?;

    let nullifier_account = banks_client.get_account(nullifier_key).await?.unwrap();
    let nullifier_data = Nullifier::try_from_slice(&nullifier_account.data)?;
    assert_eq!(nullifier_data.hash, nullifier);
    assert_eq!(nullifier_data.flow_id, flow_id);

    Ok(())
}

#[tokio::test]
async fn test_nullifier_tracking() -> Result<()> {
    let (mut banks_client, payer, recent_blockhash) = common::setup().await;
    let nullifier = [4u8; 32];
    let flow_id = 1u64;
    
    let nullifier_key = Pubkey::find_program_address(
        &[b"nullifier", &nullifier],
        &wave_verifier::id(),
    ).0;

    // First use should succeed
    let ix1 = CloudVerifierInstruction::ValidateProof {
        proof: common::create_test_proof().proof_bytes,
        public_inputs: vec![1, 2, 3],
        nullifier,
    };

    let transaction1 = Transaction::new_signed_with_payer(
        &[Instruction::new_with_borsh(
            wave_verifier::id(),
            &ix1,
            vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(flow_registry_key, false),
                AccountMeta::new(nullifier_key, false),
                AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            ],
        )],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    banks_client.process_transaction(transaction1).await?;

    // Second use should fail
    let ix2 = CloudVerifierInstruction::ValidateProof {
        proof: common::create_test_proof().proof_bytes,
        public_inputs: vec![1, 2, 3],
        nullifier,
    };

    let transaction2 = Transaction::new_signed_with_payer(
        &[Instruction::new_with_borsh(
            wave_verifier::id(),
            &ix2,
            vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(flow_registry_key, false),
                AccountMeta::new(nullifier_key, false),
                AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            ],
        )],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    let result = banks_client.process_transaction(transaction2).await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_flow_trigger() -> Result<()> {
    let (mut banks_client, payer, recent_blockhash) = common::setup().await;
    let flow_id = 1u64;
    let target_program = Keypair::new();
    
    let instruction_data = vec![1, 2, 3, 4, 5];

    let ix = CloudVerifierInstruction::TriggerFlow {
        flow_id,
        instruction_data: instruction_data.clone(),
    };

    let transaction = Transaction::new_signed_with_payer(
        &[Instruction::new_with_borsh(
            wave_verifier::id(),
            &ix,
            vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(flow_registry_key, false),
                AccountMeta::new_readonly(target_program.pubkey(), false),
            ],
        )],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    banks_client.process_transaction(transaction).await?;

    Ok(())
}

#[test]
fn test_init_registry() {
    let flow = common::create_test_flow();
    
    let instruction = WaveInstruction::InitRegistry {
        flow_id: flow.id,
        merkle_root: flow.merkle_root,
        circuit_hash: flow.circuit_hash,
        callback_program_id: flow.callback_program_id,
    };

    let authority = Pubkey::new_unique();
    let registry_account = AccountInfo::new(
        &Pubkey::new_unique(),
        true,
        true,
        &mut [0u8; 1000],
        &mut [],
        &authority,
        false,
        0,
    );

    let system_program_account = AccountInfo::new(
        &system_program::id(),
        false,
        false,
        &mut [],
        &mut [],
        &Pubkey::new_unique(),
        false,
        0,
    );

    let accounts = vec![
        AccountInfo::new(
            &authority,
            true,
            false,
            &mut [],
            &mut [],
            &Pubkey::new_unique(),
            false,
            0,
        ),
        registry_account.clone(),
        system_program_account,
    ];

    let result = wave_verifier::processor::process_instruction(
        &Pubkey::new_unique(),
        &accounts,
        &instruction.try_to_vec().unwrap(),
    );

    assert!(result.is_ok());

    let loaded_registry = FlowRegistry::load(&registry_account).unwrap();
    assert_eq!(loaded_registry.flow_id, flow.id);
    assert_eq!(loaded_registry.merkle_root, flow.merkle_root);
    assert_eq!(loaded_registry.circuit_hash, flow.circuit_hash);
}

#[test]
fn test_validate_proof() {
    let flow = common::create_test_flow();
    let proof = common::create_test_proof();
    
    let instruction = WaveInstruction::ValidateProof {
        proof: proof.proof_bytes,
        public_inputs: proof.public_inputs,
        nullifier: proof.nullifier,
    };

    let payer = Pubkey::new_unique();
    let registry_account = AccountInfo::new(
        &Pubkey::new_unique(),
        false,
        false,
        &mut [0u8; 1000],
        &mut [],
        &Pubkey::new_unique(),
        false,
        0,
    );

    let nullifier_account = AccountInfo::new(
        &Pubkey::new_unique(),
        true,
        true,
        &mut [0u8; 1000],
        &mut [],
        &Pubkey::new_unique(),
        false,
        0,
    );

    let proof_log_account = AccountInfo::new(
        &Pubkey::new_unique(),
        true,
        true,
        &mut [0u8; 1000],
        &mut [],
        &Pubkey::new_unique(),
        false,
        0,
    );

    let system_program_account = AccountInfo::new(
        &system_program::id(),
        false,
        false,
        &mut [],
        &mut [],
        &Pubkey::new_unique(),
        false,
        0,
    );

    let accounts = vec![
        AccountInfo::new(
            &payer,
            true,
            false,
            &mut [],
            &mut [],
            &Pubkey::new_unique(),
            false,
            0,
        ),
        registry_account,
        nullifier_account.clone(),
        proof_log_account.clone(),
        system_program_account,
    ];

    let result = wave_verifier::processor::process_instruction(
        &Pubkey::new_unique(),
        &accounts,
        &instruction.try_to_vec().unwrap(),
    );

    assert!(result.is_ok());

    let loaded_nullifier = Nullifier::load(&nullifier_account).unwrap();
    assert_eq!(loaded_nullifier.hash, proof.nullifier);

    let loaded_proof_log = ProofLog::load(&proof_log_account).unwrap();
    assert_eq!(loaded_proof_log.nullifier, proof.nullifier);
}

#[test]
fn test_set_root() {
    let instruction = WaveInstruction::SetRoot {
        new_root: MERKLE_ROOT_2,
    };

    let authority = Pubkey::new_unique();
    let registry_account = AccountInfo::new(
        &Pubkey::new_unique(),
        true,
        true,
        &mut [0u8; 1000],
        &mut [],
        &authority,
        false,
        0,
    );

    let accounts = vec![
        AccountInfo::new(
            &authority,
            true,
            false,
            &mut [],
            &mut [],
            &Pubkey::new_unique(),
            false,
            0,
        ),
        registry_account.clone(),
    ];

    let result = wave_verifier::processor::process_instruction(
        &Pubkey::new_unique(),
        &accounts,
        &instruction.try_to_vec().unwrap(),
    );

    assert!(result.is_ok());

    let loaded_registry = FlowRegistry::load(&registry_account).unwrap();
    assert_eq!(loaded_registry.merkle_root, Some(MERKLE_ROOT_2));
}

#[test]
fn test_trigger_flow() {
    let instruction = WaveInstruction::TriggerFlow {
        flow_id: FLOW_ID_1,
        instruction_data: vec![1, 2, 3],
    };

    let payer = Pubkey::new_unique();
    let registry_account = AccountInfo::new(
        &Pubkey::new_unique(),
        false,
        false,
        &mut [0u8; 1000],
        &mut [],
        &Pubkey::new_unique(),
        false,
        0,
    );

    let target_program = Pubkey::new_unique();
    let target_program_account = AccountInfo::new(
        &target_program,
        false,
        false,
        &mut [],
        &mut [],
        &Pubkey::new_unique(),
        false,
        0,
    );

    let accounts = vec![
        AccountInfo::new(
            &payer,
            true,
            false,
            &mut [],
            &mut [],
            &Pubkey::new_unique(),
            false,
            0,
        ),
        registry_account,
        target_program_account,
    ];

    let result = wave_verifier::processor::process_instruction(
        &Pubkey::new_unique(),
        &accounts,
        &instruction.try_to_vec().unwrap(),
    );

    assert!(result.is_ok());
} 