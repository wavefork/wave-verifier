use {
    solana_program::{
        account_info::AccountInfo,
        program_error::ProgramError,
        pubkey::Pubkey,
        clock::Clock,
        sysvar::Sysvar,
    },
    solana_program_test::*,
    solana_sdk::{
        signature::Signer,
        transaction::Transaction,
        signer::keypair::Keypair,
    },
    account_compression::{
        state::{CompressionState, GlobalCompressionConfig, CompressionAlgorithm, AccountType},
        error::CompressionError,
    },
};

mod common {
    use super::*;
    
    pub async fn setup_program_test() -> (BanksClient, Keypair, Hash) {
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

    pub fn create_test_account(size: usize) -> (Keypair, Vec<u8>) {
        let account = Keypair::new();
        let data = vec![42u8; size]; // Fill with test data
        (account, data)
    }
}

#[tokio::test]
async fn test_initialize_compression() {
    use common::*;
    
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
    use common::*;
    
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
    use common::*;
    
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
    use common::*;
    
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

#[tokio::test]
async fn test_concurrent_compression() {
    use common::*;
    
    let (mut banks_client, payer, recent_blockhash) = setup_program_test().await;
    
    // Create multiple accounts for concurrent compression
    let accounts: Vec<(Keypair, Vec<u8>)> = (0..10)
        .map(|_| create_test_account(1000))
        .collect();
        
    // Compress accounts concurrently
    let mut handles = vec![];
    
    for (account, _) in accounts {
        let handle = tokio::spawn(async move {
            let transaction = Transaction::new_signed_with_payer(
                &[account_compression::instruction::compress_account(
                    &program_id,
                    &account.pubkey(),
                    AccountType::User,
                    GlobalCompressionConfig {
                        default_algorithm: CompressionAlgorithm::Lz4,
                        min_chunk_size: 512,
                        max_chunk_size: 4096,
                        concurrent_compressions_limit: 4,
                        verify_all_compressions: true,
                        auto_decompress_on_access: false,
                    },
                )],
                Some(&payer.pubkey()),
                &[&payer],
                recent_blockhash,
            );
            
            banks_client.process_transaction(transaction).await
        });
        
        handles.push(handle);
    }
    
    // Wait for all compressions to complete
    for handle in handles {
        handle.await.unwrap().unwrap();
    }
} 