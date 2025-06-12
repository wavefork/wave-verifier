use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::AccountInfo,
        program_error::ProgramError,
        pubkey::Pubkey,
        clock::UnixTimestamp,
    },
    std::{
        io::{self, Write},
        collections::VecDeque,
    },
};

pub const COMPRESSION_HEADER_SIZE: usize = 8;
pub const MAX_UNCOMPRESSED_SIZE: usize = 10 * 1024 * 1024; // 10MB
pub const MAX_QUEUE_SIZE: usize = 1000;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct QueueMetadata {
    pub creation_time: UnixTimestamp,
    pub last_processed: UnixTimestamp,
    pub authority: Pubkey,
    pub is_locked: bool,
    pub total_items_processed: u64,
    pub compression_ratio: f64,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct CompressionQueue {
    pub metadata: QueueMetadata,
    pending_items: VecDeque<QueueItem>,
    processed_count: u64,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
struct QueueItem {
    pub data: Vec<u8>,
    pub compression_type: CompressionType,
    pub priority: u8,
    pub timestamp: UnixTimestamp,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct CompressedAccount {
    pub version: u8,
    pub original_size: u32,
    pub compression_type: CompressionType,
    pub data: Vec<u8>,
    pub metadata: AccountMetadata,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct AccountMetadata {
    pub last_compressed: UnixTimestamp,
    pub compression_count: u32,
    pub original_space: u32,
    pub saved_space: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum CompressionType {
    None = 0,
    Lz4 = 1,
    Snappy = 2,
    Zstd = 3,
}

impl CompressionQueue {
    pub fn new(authority: Pubkey) -> Self {
        Self {
            metadata: QueueMetadata {
                creation_time: 0,
                last_processed: 0,
                authority,
                is_locked: false,
                total_items_processed: 0,
                compression_ratio: 1.0,
            },
            pending_items: VecDeque::new(),
            processed_count: 0,
        }
    }

    pub fn enqueue(
        &mut self,
        data: Vec<u8>,
        compression_type: CompressionType,
        priority: u8,
    ) -> Result<(), ProgramError> {
        if self.metadata.is_locked {
            return Err(ProgramError::InvalidAccountData);
        }

        if self.pending_items.len() >= MAX_QUEUE_SIZE {
            return Err(ProgramError::InvalidArgument);
        }

        let item = QueueItem {
            data,
            compression_type,
            priority,
            timestamp: 0, // Should be set from blockchain
        };

        match priority {
            0 => self.pending_items.push_back(item),
            _ => self.pending_items.push_front(item),
        }

        Ok(())
    }

    pub fn process_next(&mut self) -> Result<Option<CompressedAccount>, ProgramError> {
        if self.pending_items.is_empty() {
            return Ok(None);
        }

        let item = self.pending_items.pop_front().unwrap();
        let original_size = item.data.len() as u32;

        let compressed_data = match item.compression_type {
            CompressionType::None => item.data,
            CompressionType::Lz4 => compress_lz4(&item.data)?,
            CompressionType::Snappy => compress_snappy(&item.data)?,
            CompressionType::Zstd => compress_zstd(&item.data)?,
        };

        let saved_space = if compressed_data.len() > item.data.len() {
            0
        } else {
            (item.data.len() - compressed_data.len()) as u32
        };

        let account = CompressedAccount {
            version: 1,
            original_size,
            compression_type: item.compression_type,
            data: compressed_data,
            metadata: AccountMetadata {
                last_compressed: 0, // Should be set from blockchain
                compression_count: 1,
                original_space: original_size,
                saved_space,
            },
        };

        self.processed_count += 1;
        self.metadata.total_items_processed += 1;
        self.update_compression_ratio(&account);

        Ok(Some(account))
    }

    fn update_compression_ratio(&mut self, account: &CompressedAccount) {
        let current_ratio = account.data.len() as f64 / account.original_size as f64;
        let weight = 0.1; // Weight for moving average
        self.metadata.compression_ratio = 
            (1.0 - weight) * self.metadata.compression_ratio + weight * current_ratio;
    }
}

impl CompressedAccount {
    pub fn new(data: &[u8], compression_type: CompressionType) -> Result<Self, ProgramError> {
        if data.len() > MAX_UNCOMPRESSED_SIZE {
            return Err(ProgramError::InvalidArgument);
        }

        let original_size = data.len() as u32;
        let compressed_data = match compression_type {
            CompressionType::None => data.to_vec(),
            CompressionType::Lz4 => compress_lz4(data)?,
            CompressionType::Snappy => compress_snappy(data)?,
            CompressionType::Zstd => compress_zstd(data)?,
        };

        let saved_space = if compressed_data.len() > data.len() {
            0
        } else {
            (data.len() - compressed_data.len()) as u32
        };

        Ok(Self {
            version: 1,
            original_size,
            compression_type,
            data: compressed_data,
            metadata: AccountMetadata {
                last_compressed: 0,
                compression_count: 1,
                original_space: original_size,
                saved_space,
            },
        })
    }

    pub fn decompress(&self) -> Result<Vec<u8>, ProgramError> {
        match self.compression_type {
            CompressionType::None => Ok(self.data.clone()),
            CompressionType::Lz4 => decompress_lz4(&self.data, self.original_size as usize),
            CompressionType::Snappy => decompress_snappy(&self.data, self.original_size as usize),
            CompressionType::Zstd => decompress_zstd(&self.data, self.original_size as usize),
        }
    }

    pub fn get_compression_ratio(&self) -> f64 {
        self.data.len() as f64 / self.original_size as f64
    }

    pub fn save(&self, account: &AccountInfo) -> Result<(), ProgramError> {
        let data = self.try_to_vec()?;
        let mut account_data = account.try_borrow_mut_data()?;
        account_data[..data.len()].copy_from_slice(&data);
        Ok(())
    }

    pub fn load(account: &AccountInfo) -> Result<Self, ProgramError> {
        let data = account.try_borrow_data()?;
        Self::try_from_slice(&data).map_err(|_| ProgramError::InvalidAccountData)
    }
}

fn compress_lz4(data: &[u8]) -> Result<Vec<u8>, ProgramError> {
    let mut encoder = lz4_flex::frame::FrameEncoder::new(Vec::new());
    encoder.write_all(data).map_err(|_| ProgramError::InvalidArgument)?;
    encoder.finish().map_err(|_| ProgramError::InvalidArgument)
}

fn decompress_lz4(compressed: &[u8], original_size: usize) -> Result<Vec<u8>, ProgramError> {
    let mut decoder = lz4_flex::frame::FrameDecoder::new(compressed);
    let mut decompressed = Vec::with_capacity(original_size);
    io::copy(&mut decoder, &mut decompressed)
        .map_err(|_| ProgramError::InvalidArgument)?;
    Ok(decompressed)
}

fn compress_snappy(data: &[u8]) -> Result<Vec<u8>, ProgramError> {
    snap::raw::Encoder::new()
        .compress_vec(data)
        .map_err(|_| ProgramError::InvalidArgument)
}

fn decompress_snappy(compressed: &[u8], original_size: usize) -> Result<Vec<u8>, ProgramError> {
    snap::raw::Decoder::new()
        .decompress_vec(compressed)
        .map_err(|_| ProgramError::InvalidArgument)
}

fn compress_zstd(data: &[u8]) -> Result<Vec<u8>, ProgramError> {
    zstd::encode_all(data, 0)
        .map_err(|_| ProgramError::InvalidArgument)
}

fn decompress_zstd(compressed: &[u8], original_size: usize) -> Result<Vec<u8>, ProgramError> {
    zstd::decode_all(compressed)
        .map_err(|_| ProgramError::InvalidArgument)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_queue() {
        let mut queue = CompressionQueue::new(Pubkey::new_unique());
        
        // Test enqueueing items
        let data1 = vec![1u8; 1000];
        let data2 = vec![2u8; 1000];
        
        assert!(queue.enqueue(data1.clone(), CompressionType::Lz4, 0).is_ok());
        assert!(queue.enqueue(data2.clone(), CompressionType::Snappy, 1).is_ok());
        
        // Process items
        let compressed1 = queue.process_next().unwrap().unwrap();
        let compressed2 = queue.process_next().unwrap().unwrap();
        
        // Verify compression
        assert!(compressed1.data.len() < data1.len());
        assert!(compressed2.data.len() < data2.len());
        
        // Verify decompression
        let decompressed1 = compressed1.decompress().unwrap();
        let decompressed2 = compressed2.decompress().unwrap();
        
        assert_eq!(decompressed1, data1);
        assert_eq!(decompressed2, data2);
    }

    #[test]
    fn test_compression_types() {
        let data = vec![1u8; 10000];
        
        // Test different compression types
        let compressed_lz4 = CompressedAccount::new(&data, CompressionType::Lz4).unwrap();
        let compressed_snappy = CompressedAccount::new(&data, CompressionType::Snappy).unwrap();
        let compressed_zstd = CompressedAccount::new(&data, CompressionType::Zstd).unwrap();
        
        // All should compress the data
        assert!(compressed_lz4.data.len() < data.len());
        assert!(compressed_snappy.data.len() < data.len());
        assert!(compressed_zstd.data.len() < data.len());
        
        // All should decompress correctly
        assert_eq!(compressed_lz4.decompress().unwrap(), data);
        assert_eq!(compressed_snappy.decompress().unwrap(), data);
        assert_eq!(compressed_zstd.decompress().unwrap(), data);
    }

    #[test]
    fn test_queue_priority() {
        let mut queue = CompressionQueue::new(Pubkey::new_unique());
        
        // Add items with different priorities
        let low_priority_data = vec![1u8; 100];
        let high_priority_data = vec![2u8; 100];
        
        queue.enqueue(low_priority_data.clone(), CompressionType::Lz4, 0).unwrap();
        queue.enqueue(high_priority_data.clone(), CompressionType::Lz4, 1).unwrap();
        
        // High priority item should be processed first
        let first = queue.process_next().unwrap().unwrap();
        let second = queue.process_next().unwrap().unwrap();
        
        assert_eq!(first.decompress().unwrap(), high_priority_data);
        assert_eq!(second.decompress().unwrap(), low_priority_data);
    }

    #[test]
    fn test_queue_limits() {
        let mut queue = CompressionQueue::new(Pubkey::new_unique());
        
        // Try to fill queue beyond capacity
        for _ in 0..=MAX_QUEUE_SIZE {
            let result = queue.enqueue(vec![0u8; 10], CompressionType::None, 0);
            if queue.pending_items.len() == MAX_QUEUE_SIZE {
                assert!(result.is_err());
                break;
            }
        }
    }
} 