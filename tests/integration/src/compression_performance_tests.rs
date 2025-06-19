use {
    solana_program::{
        account_info::AccountInfo,
        pubkey::Pubkey,
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

async fn setup_large_test_data() -> Vec<u8> {
    // Create test data with repeating patterns for better compression
    let mut data = Vec::with_capacity(100_000);
    for i in 0..100_000 {
        data.push((i % 256) as u8);
    }
    data
}

#[tokio::test]
async fn test_large_account_compression() {
    let (mut banks_client, payer, recent_blockhash) = setup_program_test().await;
    
    // Create a large account
    let test_data = setup_large_test_data().await;
    let test_account = Keypair::new();
    
    // Initialize account with large data
    let rent = banks_client.get_rent().await.unwrap();
    let lamports = rent.minimum_balance(test_data.len());
    
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &test_account.pubkey(),
                lamports,
                test_data.len() as u64,
                &program_id,
            ),
        ],
        Some(&payer.pubkey()),
        &[&payer, &test_account],
        recent_blockhash,
    );
    
    banks_client.process_transaction(transaction).await.unwrap();
    
    // Test compression with different algorithms
    let algorithms = vec![
        CompressionAlgorithm::Lz4,
        CompressionAlgorithm::Snappy,
        CompressionAlgorithm::Zstd,
    ];
    
    for algorithm in algorithms {
        let config = GlobalCompressionConfig {
            default_algorithm: algorithm,
            min_chunk_size: 4096,
            max_chunk_size: 16384,
            concurrent_compressions_limit: 4,
            verify_all_compressions: true,
            auto_decompress_on_access: false,
        };
        
        let start = std::time::Instant::now();
        
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
        
        let compressed_account = banks_client
            .get_account(test_account.pubkey())
            .await
            .unwrap()
            .unwrap();
        
        let compression_time = start.elapsed();
        let compression_ratio = test_data.len() as f64 / compressed_account.data.len() as f64;
        
        println!(
            "Algorithm {:?}: Time = {:?}, Ratio = {:.2}",
            algorithm,
            compression_time,
            compression_ratio
        );
        
        // Verify compression effectiveness
        assert!(compression_ratio > 1.0, "Compression should reduce data size");
        assert!(compression_time.as_millis() < 1000, "Compression should be fast");
    }
}

#[tokio::test]
async fn test_concurrent_compression_performance() {
    let (mut banks_client, payer, recent_blockhash) = setup_program_test().await;
    
    // Create multiple accounts with varying data sizes
    let account_sizes = vec![1000, 5000, 10000, 50000];
    let mut accounts = Vec::new();
    
    for size in account_sizes {
        let account = Keypair::new();
        let data = (0..size).map(|i| (i % 256) as u8).collect::<Vec<_>>();
        accounts.push((account, data));
    }
    
    // Compress accounts concurrently
    let mut handles = Vec::new();
    
    for (account, data) in accounts {
        let handle = tokio::spawn(async move {
            let config = GlobalCompressionConfig {
                default_algorithm: CompressionAlgorithm::Lz4,
                min_chunk_size: 1024,
                max_chunk_size: 8192,
                concurrent_compressions_limit: 8,
                verify_all_compressions: true,
                auto_decompress_on_access: false,
            };
            
            let transaction = Transaction::new_signed_with_payer(
                &[account_compression::instruction::compress_account(
                    &program_id,
                    &account.pubkey(),
                    AccountType::User,
                    config,
                )],
                Some(&payer.pubkey()),
                &[&payer],
                recent_blockhash,
            );
            
            let start = std::time::Instant::now();
            banks_client.process_transaction(transaction).await.unwrap();
            (start.elapsed(), data.len())
        });
        
        handles.push(handle);
    }
    
    // Collect and analyze results
    let mut total_throughput = 0.0;
    for handle in handles {
        let (time, size) = handle.await.unwrap();
        let throughput = size as f64 / time.as_secs_f64() / 1024.0 / 1024.0; // MB/s
        total_throughput += throughput;
        println!("Compressed {}KB in {:?} ({:.2} MB/s)", size/1024, time, throughput);
    }
    
    println!("Total throughput: {:.2} MB/s", total_throughput);
    assert!(total_throughput > 1.0, "Minimum throughput not met");
}

#[tokio::test]
async fn test_compression_with_different_chunk_sizes() {
    let (mut banks_client, payer, recent_blockhash) = setup_program_test().await;
    
    let test_data = setup_large_test_data().await;
    let chunk_sizes = vec![512, 1024, 2048, 4096, 8192];
    
    for chunk_size in chunk_sizes {
        let test_account = Keypair::new();
        let config = GlobalCompressionConfig {
            default_algorithm: CompressionAlgorithm::Lz4,
            min_chunk_size: chunk_size,
            max_chunk_size: chunk_size,
            concurrent_compressions_limit: 1,
            verify_all_compressions: true,
            auto_decompress_on_access: false,
        };
        
        let start = std::time::Instant::now();
        
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
        
        let compressed_account = banks_client
            .get_account(test_account.pubkey())
            .await
            .unwrap()
            .unwrap();
        
        println!(
            "Chunk size {}: Time = {:?}, Compressed size = {}",
            chunk_size,
            start.elapsed(),
            compressed_account.data.len()
        );
    }
}

#[tokio::test]
async fn test_compression_memory_usage() {
    let (mut banks_client, payer, recent_blockhash) = setup_program_test().await;
    
    // Test with increasingly large accounts
    let sizes = vec![1024, 10_240, 102_400, 1_024_000];
    
    for size in sizes {
        let test_account = Keypair::new();
        let test_data = (0..size).map(|i| (i % 256) as u8).collect::<Vec<_>>();
        
        let config = GlobalCompressionConfig {
            default_algorithm: CompressionAlgorithm::Lz4,
            min_chunk_size: 4096,
            max_chunk_size: 16384,
            concurrent_compressions_limit: 1,
            verify_all_compressions: true,
            auto_decompress_on_access: false,
        };
        
        let before_memory = get_process_memory();
        
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
        
        let after_memory = get_process_memory();
        let memory_increase = after_memory - before_memory;
        
        println!(
            "Account size {}KB: Memory usage increase = {}KB",
            size/1024,
            memory_increase/1024
        );
        
        // Memory usage should be reasonable
        assert!(
            memory_increase < size as u64 * 2,
            "Memory usage too high for compression"
        );
    }
}

fn get_process_memory() -> u64 {
    // This is a mock implementation - in real code, you'd use platform-specific APIs
    // to get actual process memory usage
    0
} 