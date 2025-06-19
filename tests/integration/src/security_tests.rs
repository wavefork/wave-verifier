use {
    solana_program::{
        account_info::AccountInfo,
        pubkey::Pubkey,
        program_error::ProgramError,
        clock::Clock,
    },
    solana_program_test::*,
    solana_sdk::{
        signature::Signer,
        transaction::Transaction,
        signer::keypair::Keypair,
    },
};

use super::*;

#[tokio::test]
async fn test_unauthorized_compression() {
    let (mut banks_client, payer, recent_blockhash) = setup_program_test().await;
    
    // Create test account
    let test_account = Keypair::new();
    let unauthorized_user = Keypair::new();
    
    // Try to compress with unauthorized user
    let config = GlobalCompressionConfig {
        default_algorithm: CompressionAlgorithm::Lz4,
        min_chunk_size: 1024,
        max_chunk_size: 4096,
        concurrent_compressions_limit: 1,
        verify_all_compressions: true,
        auto_decompress_on_access: false,
    };
    
    let transaction = Transaction::new_signed_with_payer(
        &[account_compression::instruction::compress_account(
            &program_id,
            &test_account.pubkey(),
            AccountType::User,
            config,
        )],
        Some(&unauthorized_user.pubkey()),
        &[&unauthorized_user],
        recent_blockhash,
    );
    
    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_malformed_data_compression() {
    let (mut banks_client, payer, recent_blockhash) = setup_program_test().await;
    
    // Create account with malformed data
    let test_account = Keypair::new();
    let malformed_data = vec![0xFF; 1000]; // All 0xFF bytes
    
    let rent = banks_client.get_rent().await.unwrap();
    let lamports = rent.minimum_balance(malformed_data.len());
    
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &test_account.pubkey(),
                lamports,
                malformed_data.len() as u64,
                &program_id,
            ),
        ],
        Some(&payer.pubkey()),
        &[&payer, &test_account],
        recent_blockhash,
    );
    
    banks_client.process_transaction(transaction).await.unwrap();
    
    // Try to compress malformed data
    let config = GlobalCompressionConfig {
        default_algorithm: CompressionAlgorithm::Lz4,
        min_chunk_size: 1024,
        max_chunk_size: 4096,
        concurrent_compressions_limit: 1,
        verify_all_compressions: true,
        auto_decompress_on_access: false,
    };
    
    let transaction = Transaction::new_signed_with_payer(
        &[account_compression::instruction::compress_account(
            &program_id,
            &test_account.pubkey(),
            AccountType::User,
            config,
        )],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    
    // Should handle malformed data gracefully
    banks_client.process_transaction(transaction).await.unwrap();
}

#[tokio::test]
async fn test_compression_buffer_overflow() {
    let (mut banks_client, payer, recent_blockhash) = setup_program_test().await;
    
    // Create account with data size at the limit
    let test_account = Keypair::new();
    let max_size = 10_000_000; // 10MB
    let data = vec![42u8; max_size];
    
    let rent = banks_client.get_rent().await.unwrap();
    let lamports = rent.minimum_balance(data.len());
    
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &test_account.pubkey(),
                lamports,
                data.len() as u64,
                &program_id,
            ),
        ],
        Some(&payer.pubkey()),
        &[&payer, &test_account],
        recent_blockhash,
    );
    
    banks_client.process_transaction(transaction).await.unwrap();
    
    // Try to compress with small buffer size
    let config = GlobalCompressionConfig {
        default_algorithm: CompressionAlgorithm::Lz4,
        min_chunk_size: 1024,
        max_chunk_size: 2048, // Too small for the data
        concurrent_compressions_limit: 1,
        verify_all_compressions: true,
        auto_decompress_on_access: false,
    };
    
    let transaction = Transaction::new_signed_with_payer(
        &[account_compression::instruction::compress_account(
            &program_id,
            &test_account.pubkey(),
            AccountType::User,
            config,
        )],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    
    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_double_compression() {
    let (mut banks_client, payer, recent_blockhash) = setup_program_test().await;
    
    // Create and compress account
    let test_account = Keypair::new();
    let data = vec![1, 2, 3, 4, 5];
    
    let config = GlobalCompressionConfig {
        default_algorithm: CompressionAlgorithm::Lz4,
        min_chunk_size: 1024,
        max_chunk_size: 4096,
        concurrent_compressions_limit: 1,
        verify_all_compressions: true,
        auto_decompress_on_access: false,
    };
    
    // First compression
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
    
    // Try second compression
    let transaction = Transaction::new_signed_with_payer(
        &[account_compression::instruction::compress_account(
            &program_id,
            &test_account.pubkey(),
            AccountType::User,
            config,
        )],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    
    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_compression_data_integrity() {
    let (mut banks_client, payer, recent_blockhash) = setup_program_test().await;
    
    // Create test data with specific patterns
    let mut test_data = Vec::new();
    for i in 0..1000 {
        test_data.extend_from_slice(&[
            0xFF, 0x00, 0xFF, 0x00, // Alternating pattern
            i as u8, (i >> 8) as u8, // Counter
            0xAA, 0x55, // Fixed pattern
        ]);
    }
    
    let test_account = Keypair::new();
    
    // Compress data
    let config = GlobalCompressionConfig {
        default_algorithm: CompressionAlgorithm::Lz4,
        min_chunk_size: 1024,
        max_chunk_size: 4096,
        concurrent_compressions_limit: 1,
        verify_all_compressions: true,
        auto_decompress_on_access: false,
    };
    
    let transaction = Transaction::new_signed_with_payer(
        &[account_compression::instruction::compress_account(
            &program_id,
            &test_account.pubkey(),
            AccountType::User,
            config,
        )],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    
    banks_client.process_transaction(transaction).await.unwrap();
    
    // Decompress and verify
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
    
    let decompressed_account = banks_client
        .get_account(test_account.pubkey())
        .await
        .unwrap()
        .unwrap();
    
    assert_eq!(decompressed_account.data, test_data);
}

#[tokio::test]
async fn test_compression_with_invalid_config() {
    let (mut banks_client, payer, recent_blockhash) = setup_program_test().await;
    
    let test_account = Keypair::new();
    
    // Test various invalid configurations
    let invalid_configs = vec![
        GlobalCompressionConfig {
            default_algorithm: CompressionAlgorithm::Lz4,
            min_chunk_size: 0, // Invalid: zero chunk size
            max_chunk_size: 4096,
            concurrent_compressions_limit: 1,
            verify_all_compressions: true,
            auto_decompress_on_access: false,
        },
        GlobalCompressionConfig {
            default_algorithm: CompressionAlgorithm::Lz4,
            min_chunk_size: 4096,
            max_chunk_size: 1024, // Invalid: max < min
            concurrent_compressions_limit: 1,
            verify_all_compressions: true,
            auto_decompress_on_access: false,
        },
        GlobalCompressionConfig {
            default_algorithm: CompressionAlgorithm::Lz4,
            min_chunk_size: 1024,
            max_chunk_size: 4096,
            concurrent_compressions_limit: 0, // Invalid: zero concurrent limit
            verify_all_compressions: true,
            auto_decompress_on_access: false,
        },
    ];
    
    for config in invalid_configs {
        let transaction = Transaction::new_signed_with_payer(
            &[account_compression::instruction::compress_account(
                &program_id,
                &test_account.pubkey(),
                AccountType::User,
                config,
            )],
            Some(&payer.pubkey()),
            &[&payer],
            recent_blockhash,
        );
        
        let result = banks_client.process_transaction(transaction).await;
        assert!(result.is_err());
    }
} 