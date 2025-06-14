use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        program_error::ProgramError,
        pubkey::Pubkey,
        clock::UnixTimestamp,
    },
    std::{
        collections::{hash_map::DefaultHasher, HashMap},
        hash::{Hash, Hasher},
    },
};

const BUCKET_SIZE: usize = 32;
const DEFAULT_CAPACITY: usize = 1024;
const MAX_ROLLOVER_ITEMS: usize = 100;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct StateMetadata {
    pub creation_time: UnixTimestamp,
    pub last_modified: UnixTimestamp,
    pub authority: Pubkey,
    pub is_frozen: bool,
    pub total_operations: u64,
    pub rollover_count: u32,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct OnChainHashSet {
    buckets: Vec<Bucket>,
    item_count: u32,
    capacity: usize,
    metadata: StateMetadata,
    rollover_buffer: RolloverBuffer,
    operation_log: OperationLog,
}

#[derive(Debug, Default, BorshSerialize, BorshDeserialize)]
struct Bucket {
    items: Vec<[u8; 32]>,
    last_modified: UnixTimestamp,
    operation_count: u32,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
struct RolloverBuffer {
    items: Vec<[u8; 32]>,
    source_buckets: Vec<usize>,
    is_active: bool,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
struct OperationLog {
    operations: Vec<Operation>,
    last_checkpoint: u64,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
struct Operation {
    op_type: OperationType,
    item: [u8; 32],
    timestamp: UnixTimestamp,
    bucket_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, BorshSerialize, BorshDeserialize)]
enum OperationType {
    Insert,
    Remove,
    Rollover,
    Checkpoint,
}

impl OnChainHashSet {
    pub fn new(capacity: Option<usize>, authority: Pubkey) -> Self {
        let capacity = capacity.unwrap_or(DEFAULT_CAPACITY);
        let bucket_count = (capacity + BUCKET_SIZE - 1) / BUCKET_SIZE;
        
        Self {
            buckets: vec![Bucket::default(); bucket_count],
            item_count: 0,
            capacity,
            metadata: StateMetadata {
                creation_time: 0,
                last_modified: 0,
                authority,
                is_frozen: false,
                total_operations: 0,
                rollover_count: 0,
            },
            rollover_buffer: RolloverBuffer {
                items: Vec::with_capacity(MAX_ROLLOVER_ITEMS),
                source_buckets: Vec::with_capacity(MAX_ROLLOVER_ITEMS),
                is_active: false,
            },
            operation_log: OperationLog {
                operations: Vec::new(),
                last_checkpoint: 0,
            },
        }
    }

    pub fn insert(&mut self, item: &[u8; 32], timestamp: UnixTimestamp) -> Result<bool, ProgramError> {
        if self.metadata.is_frozen {
            return Err(ProgramError::InvalidAccountData);
        }

        if self.item_count as usize >= self.capacity {
            return Err(ProgramError::InvalidArgument);
        }

        let bucket_idx = self.get_bucket_index(item);
        let bucket = &mut self.buckets[bucket_idx];

        // Check if item already exists
        if bucket.items.contains(item) {
            return Ok(false);
        }

        // Insert new item
        bucket.items.push(*item);
        bucket.last_modified = timestamp;
        bucket.operation_count += 1;
        self.item_count += 1;
        
        // Log operation
        self.log_operation(Operation {
            op_type: OperationType::Insert,
            item: *item,
            timestamp,
            bucket_index: bucket_idx,
        });

        // Check if bucket needs rollover
        if bucket.items.len() >= BUCKET_SIZE {
            self.prepare_rollover(bucket_idx)?;
        }

        Ok(true)
    }

    pub fn remove(&mut self, item: &[u8; 32], timestamp: UnixTimestamp) -> Result<bool, ProgramError> {
        if self.metadata.is_frozen {
            return Err(ProgramError::InvalidAccountData);
        }

        let bucket_idx = self.get_bucket_index(item);
        let bucket = &mut self.buckets[bucket_idx];

        if let Some(pos) = bucket.items.iter().position(|x| x == item) {
            bucket.items.swap_remove(pos);
            bucket.last_modified = timestamp;
            bucket.operation_count += 1;
            self.item_count -= 1;

            // Log operation
            self.log_operation(Operation {
                op_type: OperationType::Remove,
                item: *item,
                timestamp,
                bucket_index: bucket_idx,
            });

            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn contains(&self, item: &[u8; 32]) -> bool {
        let bucket_idx = self.get_bucket_index(item);
        self.buckets[bucket_idx].items.contains(item)
    }

    pub fn process_rollover(&mut self, timestamp: UnixTimestamp) -> Result<(), ProgramError> {
        if !self.rollover_buffer.is_active {
            return Ok(());
        }

        // Create a temporary map for rehashing
        let mut new_locations: HashMap<[u8; 32], usize> = HashMap::new();

        // Recalculate bucket indices for all items in rollover buffer
        for item in &self.rollover_buffer.items {
            let new_bucket_idx = self.get_bucket_index(item);
            new_locations.insert(*item, new_bucket_idx);
        }

        // Move items to their new buckets
        for (item, new_bucket_idx) in new_locations {
            let bucket = &mut self.buckets[new_bucket_idx];
            bucket.items.push(item);
            bucket.last_modified = timestamp;
            bucket.operation_count += 1;
        }

        // Log rollover operation
        self.log_operation(Operation {
            op_type: OperationType::Rollover,
            item: [0u8; 32],
            timestamp,
            bucket_index: 0,
        });

        // Clear rollover buffer
        self.rollover_buffer.items.clear();
        self.rollover_buffer.source_buckets.clear();
        self.rollover_buffer.is_active = false;
        self.metadata.rollover_count += 1;

        Ok(())
    }

    pub fn checkpoint(&mut self, timestamp: UnixTimestamp) -> Result<(), ProgramError> {
        // Process any pending rollovers first
        if self.rollover_buffer.is_active {
            self.process_rollover(timestamp)?;
        }

        // Log checkpoint operation
        self.log_operation(Operation {
            op_type: OperationType::Checkpoint,
            item: [0u8; 32],
            timestamp,
            bucket_index: 0,
        });

        // Update checkpoint
        self.operation_log.last_checkpoint = self.metadata.total_operations;
        
        // Clear old operations
        self.operation_log.operations.clear();

        Ok(())
    }

    fn prepare_rollover(&mut self, bucket_idx: usize) -> Result<(), ProgramError> {
        if self.rollover_buffer.is_active {
            return Ok(());
        }

        let bucket = &mut self.buckets[bucket_idx];
        
        // Move half of the items to rollover buffer
        let items_to_move = bucket.items.len() / 2;
        let mut items: Vec<[u8; 32]> = bucket.items.drain(..items_to_move).collect();
        
        self.rollover_buffer.items.append(&mut items);
        self.rollover_buffer.source_buckets.push(bucket_idx);
        self.rollover_buffer.is_active = true;

        Ok(())
    }

    fn log_operation(&mut self, operation: Operation) {
        self.operation_log.operations.push(operation);
        self.metadata.total_operations += 1;
    }

    fn get_bucket_index(&self, item: &[u8; 32]) -> usize {
        let mut hasher = DefaultHasher::new();
        item.hash(&mut hasher);
        (hasher.finish() as usize) % self.buckets.len()
    }

    pub fn get_bucket_stats(&self) -> Vec<BucketStats> {
        self.buckets
            .iter()
            .enumerate()
            .map(|(idx, bucket)| BucketStats {
                bucket_index: idx,
                item_count: bucket.items.len(),
                operation_count: bucket.operation_count,
                last_modified: bucket.last_modified,
            })
            .collect()
    }

    pub fn get_operation_history(&self) -> &[Operation] {
        &self.operation_log.operations
    }
}

#[derive(Debug)]
pub struct BucketStats {
    pub bucket_index: usize,
    pub item_count: usize,
    pub operation_count: u32,
    pub last_modified: UnixTimestamp,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_set() -> OnChainHashSet {
        OnChainHashSet::new(Some(128), Pubkey::new_unique())
    }

    #[test]
    fn test_basic_operations() {
        let mut set = create_test_set();
        let timestamp = 1000;
        
        let item1 = [1u8; 32];
        let item2 = [2u8; 32];
        let item3 = [3u8; 32];

        // Test insertions
        assert!(set.insert(&item1, timestamp).unwrap());
        assert!(set.insert(&item2, timestamp).unwrap());
        assert!(set.insert(&item3, timestamp).unwrap());
        assert_eq!(set.item_count, 3);

        // Test contains
        assert!(set.contains(&item1));
        assert!(set.contains(&item2));
        assert!(set.contains(&item3));
        assert!(!set.contains(&[4u8; 32]));

        // Test duplicate insertion
        assert!(!set.insert(&item1, timestamp).unwrap());
        assert_eq!(set.item_count, 3);

        // Test removal
        assert!(set.remove(&item2, timestamp).unwrap());
        assert_eq!(set.item_count, 2);
        assert!(!set.contains(&item2));

        // Verify operation log
        let history = set.get_operation_history();
        assert_eq!(history.len(), 5); // 3 inserts, 1 failed insert, 1 remove
    }

    #[test]
    fn test_rollover() {
        let mut set = create_test_set();
        let timestamp = 1000;
        
        // Fill a bucket to trigger rollover
        let mut items = Vec::new();
        for i in 0..BUCKET_SIZE {
            let mut item = [0u8; 32];
            item[0] = i as u8;
            items.push(item);
        }

        // Insert items to trigger rollover
        for item in &items {
            set.insert(item, timestamp).unwrap();
        }

        // Verify rollover buffer is active
        assert!(set.rollover_buffer.is_active);
        
        // Process rollover
        set.process_rollover(timestamp).unwrap();
        
        // Verify items are still accessible
        for item in &items {
            assert!(set.contains(item));
        }
    }

    #[test]
    fn test_checkpoint() {
        let mut set = create_test_set();
        let timestamp = 1000;
        
        // Perform some operations
        let item = [1u8; 32];
        set.insert(&item, timestamp).unwrap();
        set.remove(&item, timestamp).unwrap();
        
        // Create checkpoint
        set.checkpoint(timestamp).unwrap();
        
        // Verify operation log is cleared
        assert!(set.get_operation_history().is_empty());
        assert_eq!(set.operation_log.last_checkpoint, 2);
    }

    #[test]
    fn test_frozen_state() {
        let mut set = create_test_set();
        let timestamp = 1000;
        
        // Freeze the set
        set.metadata.is_frozen = true;
        
        // Attempts to modify should fail
        let item = [1u8; 32];
        assert!(set.insert(&item, timestamp).is_err());
        assert!(set.remove(&item, timestamp).is_err());
        
        // Contains should still work
        assert!(!set.contains(&item));
    }
} 