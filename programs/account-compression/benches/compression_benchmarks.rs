#![feature(test)]
extern crate test;

use {
    account_compression::{
        state::{CompressionState, GlobalCompressionConfig, CompressionAlgorithm, AccountType},
        error::CompressionError,
    },
    solana_program::{
        account_info::AccountInfo,
        pubkey::Pubkey,
    },
    test::Bencher,
};

fn setup_test_data(size: usize) -> Vec<u8> {
    // Create test data with some patterns to make compression meaningful
    let mut data = Vec::with_capacity(size);
    for i in 0..size {
        data.push((i % 256) as u8);
    }
    data
}

#[bench]
fn bench_lz4_compression(b: &mut Bencher) {
    let test_data = setup_test_data(10000);
    
    b.iter(|| {
        let config = GlobalCompressionConfig {
            default_algorithm: CompressionAlgorithm::Lz4,
            min_chunk_size: 512,
            max_chunk_size: 4096,
            concurrent_compressions_limit: 1,
            verify_all_compressions: false,
            auto_decompress_on_access: false,
        };
        
        let mut encoder = lz4_flex::frame::FrameEncoder::new(Vec::new());
        std::io::Write::write_all(&mut encoder, &test_data).unwrap();
        encoder.finish().unwrap()
    });
}

#[bench]
fn bench_snappy_compression(b: &mut Bencher) {
    let test_data = setup_test_data(10000);
    
    b.iter(|| {
        snap::raw::Encoder::new()
            .compress_vec(&test_data)
            .unwrap()
    });
}

#[bench]
fn bench_zstd_compression(b: &mut Bencher) {
    let test_data = setup_test_data(10000);
    
    b.iter(|| {
        zstd::encode_all(&test_data, 3).unwrap()
    });
}

#[bench]
fn bench_compression_with_verification(b: &mut Bencher) {
    let test_data = setup_test_data(10000);
    
    b.iter(|| {
        // Compress
        let compressed = zstd::encode_all(&test_data, 3).unwrap();
        
        // Calculate hash
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&compressed);
        let hash = hasher.finalize();
        
        // Decompress and verify
        let decompressed = zstd::decode_all(&compressed).unwrap();
        assert_eq!(decompressed, test_data);
        
        (compressed, hash)
    });
}

#[bench]
fn bench_batch_compression(b: &mut Bencher) {
    let batch_size = 100;
    let accounts: Vec<Vec<u8>> = (0..batch_size)
        .map(|_| setup_test_data(1000))
        .collect();
    
    b.iter(|| {
        let mut compressed_accounts = Vec::with_capacity(batch_size);
        for account_data in &accounts {
            let compressed = lz4_flex::compress(account_data);
            compressed_accounts.push(compressed);
        }
        compressed_accounts
    });
}

#[bench]
fn bench_compression_queue_processing(b: &mut Bencher) {
    use std::collections::VecDeque;
    
    let queue_size = 50;
    let mut compression_queue = VecDeque::with_capacity(queue_size);
    
    // Fill queue with test accounts
    for _ in 0..queue_size {
        compression_queue.push_back(setup_test_data(1000));
    }
    
    b.iter(|| {
        let mut results = Vec::with_capacity(queue_size);
        while let Some(account_data) = compression_queue.pop_front() {
            let compressed = lz4_flex::compress(&account_data);
            results.push(compressed);
        }
        results
    });
}

#[bench]
fn bench_compression_algorithms_comparison(b: &mut Bencher) {
    let test_data = setup_test_data(10000);
    
    b.iter(|| {
        // LZ4
        let lz4_compressed = {
            let mut encoder = lz4_flex::frame::FrameEncoder::new(Vec::new());
            std::io::Write::write_all(&mut encoder, &test_data).unwrap();
            encoder.finish().unwrap()
        };
        
        // Snappy
        let snappy_compressed = snap::raw::Encoder::new()
            .compress_vec(&test_data)
            .unwrap();
        
        // Zstd
        let zstd_compressed = zstd::encode_all(&test_data, 3).unwrap();
        
        // Compare compression ratios
        let lz4_ratio = test_data.len() as f64 / lz4_compressed.len() as f64;
        let snappy_ratio = test_data.len() as f64 / snappy_compressed.len() as f64;
        let zstd_ratio = test_data.len() as f64 / zstd_compressed.len() as f64;
        
        (lz4_ratio, snappy_ratio, zstd_ratio)
    });
}

#[bench]
fn bench_concurrent_compression(b: &mut Bencher) {
    use rayon::prelude::*;
    
    let num_accounts = 100;
    let accounts: Vec<Vec<u8>> = (0..num_accounts)
        .map(|_| setup_test_data(1000))
        .collect();
    
    b.iter(|| {
        accounts.par_iter()
            .map(|data| {
                let mut encoder = lz4_flex::frame::FrameEncoder::new(Vec::new());
                std::io::Write::write_all(&mut encoder, data).unwrap();
                encoder.finish().unwrap()
            })
            .collect::<Vec<_>>()
    });
}

#[bench]
fn bench_compression_with_different_chunk_sizes(b: &mut Bencher) {
    let test_data = setup_test_data(10000);
    let chunk_sizes = [256, 512, 1024, 2048, 4096];
    
    b.iter(|| {
        chunk_sizes.iter().map(|&chunk_size| {
            let chunks: Vec<_> = test_data.chunks(chunk_size).collect();
            let mut compressed_chunks = Vec::with_capacity(chunks.len());
            
            for chunk in chunks {
                let compressed = lz4_flex::compress(chunk);
                compressed_chunks.push(compressed);
            }
            
            (chunk_size, compressed_chunks)
        }).collect::<Vec<_>>()
    });
} 