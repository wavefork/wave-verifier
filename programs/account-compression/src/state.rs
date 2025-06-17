use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        program_error::ProgramError,
        program_pack::{IsInitialized, Pack, Sealed},
        pubkey::Pubkey,
    },
};

use crate::error::CompressionError;

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct CompressionState {
    pub is_initialized: bool,
    pub authority: Pubkey,
    pub max_depth: u32,
    pub max_buffer_size: u32,
    pub total_accounts_compressed: u64,
    pub total_bytes_saved: u64,
    pub compression_stats: GlobalCompressionStats,
    pub config: GlobalCompressionConfig,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct GlobalCompressionStats {
    pub total_compressions: u64,
    pub total_decompressions: u64,
    pub average_compression_ratio: f64,
    pub best_compression_ratio: f64,
    pub worst_compression_ratio: f64,
    pub total_compression_time_ms: u64,
    pub average_compression_time_ms: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct GlobalCompressionConfig {
    pub default_algorithm: CompressionAlgorithm,
    pub min_chunk_size: u32,
    pub max_chunk_size: u32,
    pub concurrent_compressions_limit: u32,
    pub verify_all_compressions: bool,
    pub auto_decompress_on_access: bool,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub enum CompressionAlgorithm {
    Lz4,
    Snappy,
    Zstd,
}

impl Sealed for CompressionState {}

impl IsInitialized for CompressionState {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for CompressionState {
    const LEN: usize = 1024; // Fixed size for the state account

    fn pack_into_slice(&self, dst: &mut [u8]) -> Result<(), ProgramError> {
        let mut slice = dst;
        self.serialize(&mut slice).map_err(|_| CompressionError::BufferOverflow.into())
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(src).map_err(|_| CompressionError::InvalidAccountState.into())
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CompressedAccountMetadata {
    pub account_type: AccountType,
    pub original_size: u64,
    pub compressed_size: u64,
    pub compression_algorithm: CompressionAlgorithm,
    pub compression_level: u8,
    pub last_accessed: i64,
    pub access_count: u64,
    pub compression_time_ms: u64,
    pub verification_hash: [u8; 32],
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub enum AccountType {
    User,
    Token,
    NFT,
    Program,
}

impl CompressedAccountMetadata {
    pub fn get_compression_ratio(&self) -> f64 {
        if self.compressed_size == 0 {
            return 1.0;
        }
        self.original_size as f64 / self.compressed_size as f64
    }

    pub fn is_compression_effective(&self) -> bool {
        self.get_compression_ratio() > 1.0
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CompressionQueue {
    pub head: u32,
    pub tail: u32,
    pub size: u32,
    pub max_size: u32,
    pub accounts: Vec<Pubkey>,
}

impl CompressionQueue {
    pub fn new(max_size: u32) -> Self {
        Self {
            head: 0,
            tail: 0,
            size: 0,
            max_size,
            accounts: Vec::with_capacity(max_size as usize),
        }
    }

    pub fn enqueue(&mut self, account: Pubkey) -> Result<(), CompressionError> {
        if self.size >= self.max_size {
            return Err(CompressionError::BufferOverflow);
        }

        self.accounts.push(account);
        self.size += 1;
        self.tail = (self.tail + 1) % self.max_size;
        Ok(())
    }

    pub fn dequeue(&mut self) -> Option<Pubkey> {
        if self.size == 0 {
            return None;
        }

        let account = self.accounts.remove(self.head as usize);
        self.size -= 1;
        self.head = (self.head + 1) % self.max_size;
        Some(account)
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn is_full(&self) -> bool {
        self.size == self.max_size
    }
} 