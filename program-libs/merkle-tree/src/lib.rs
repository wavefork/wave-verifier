use {
    borsh::{BorshDeserialize, BorshSerialize},
    sha2::{Digest, Sha256},
    solana_program::{
        program_error::ProgramError,
        pubkey::Pubkey,
        clock::UnixTimestamp,
    },
    std::{
        collections::{VecDeque, HashMap},
        sync::Arc,
    },
};

pub const MAX_TREE_DEPTH: usize = 32;
pub const EMPTY_SLICE: [u8; 32] = [0u8; 32];
pub const MAX_BATCH_SIZE: usize = 1024;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct TreeMetadata {
    pub creation_time: UnixTimestamp,
    pub last_modified: UnixTimestamp,
    pub authority: Pubkey,
    pub is_finalized: bool,
    pub max_leaf_size: u32,
    pub compression_enabled: bool,
    pub version: u8,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct BatchOperation {
    pub sequence_number: u64,
    pub leaves: Vec<[u8; 32]>,
    pub metadata: BatchMetadata,
    pub status: BatchStatus,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct BatchMetadata {
    pub timestamp: UnixTimestamp,
    pub processor: Pubkey,
    pub priority: u8,
    pub batch_type: BatchType,
}

#[derive(Debug, Clone, Copy, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum BatchType {
    Standard,
    Priority,
    Rollover,
}

#[derive(Debug, Clone, Copy, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum BatchStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct MerkleTree {
    pub root: [u8; 32],
    pub leaf_count: u64,
    nodes: Vec<[u8; 32]>,
    depth: usize,
    metadata: TreeMetadata,
    pending_batches: VecDeque<BatchOperation>,
    processed_batches: HashMap<u64, BatchOperation>,
}

impl MerkleTree {
    pub fn new(
        depth: usize,
        authority: Pubkey,
        max_leaf_size: u32,
        compression_enabled: bool,
    ) -> Self {
        assert!(depth <= MAX_TREE_DEPTH, "Tree depth exceeds maximum");
        let capacity = (1 << (depth + 1)) - 1;
        
        let metadata = TreeMetadata {
            creation_time: 0, // Should be set from blockchain
            last_modified: 0,
            authority,
            is_finalized: false,
            max_leaf_size,
            compression_enabled,
            version: 1,
        };
        
        Self {
            root: EMPTY_SLICE,
            leaf_count: 0,
            nodes: vec![EMPTY_SLICE; capacity],
            depth,
            metadata,
            pending_batches: VecDeque::new(),
            processed_batches: HashMap::new(),
        }
    }

    pub fn create_batch(
        &mut self,
        leaves: Vec<[u8; 32]>,
        processor: Pubkey,
        batch_type: BatchType,
    ) -> Result<u64, ProgramError> {
        if leaves.len() > MAX_BATCH_SIZE {
            return Err(ProgramError::InvalidArgument);
        }

        let sequence_number = self.get_next_sequence_number();
        let batch = BatchOperation {
            sequence_number,
            leaves,
            metadata: BatchMetadata {
                timestamp: 0, // Should be set from blockchain
                processor,
                priority: match batch_type {
                    BatchType::Priority => 1,
                    BatchType::Rollover => 2,
                    BatchType::Standard => 0,
                },
                batch_type,
            },
            status: BatchStatus::Pending,
        };

        self.pending_batches.push_back(batch);
        Ok(sequence_number)
    }

    pub fn process_next_batch(&mut self) -> Result<Option<u64>, ProgramError> {
        if let Some(mut batch) = self.pending_batches.pop_front() {
            batch.status = BatchStatus::Processing;
            
            for leaf in &batch.leaves {
                self.insert(leaf)?;
            }

            batch.status = BatchStatus::Completed;
            let sequence_number = batch.sequence_number;
            self.processed_batches.insert(sequence_number, batch);
            
            Ok(Some(sequence_number))
        } else {
            Ok(None)
        }
    }

    pub fn insert(&mut self, leaf: &[u8; 32]) -> Result<u64, ProgramError> {
        if self.leaf_count as usize >= 1 << self.depth {
            return Err(ProgramError::InvalidArgument);
        }

        let leaf_index = self.leaf_count as usize;
        let node_index = self.get_leaf_node_index(leaf_index);
        
        self.nodes[node_index] = *leaf;
        self.update_path_to_root(node_index);
        
        self.leaf_count += 1;
        self.metadata.last_modified = 0; // Should be set from blockchain
        
        Ok(self.leaf_count - 1)
    }

    pub fn verify(&self, leaf: &[u8; 32], proof: &[[u8; 32]], index: u64) -> bool {
        if proof.len() != self.depth {
            return false;
        }

        let mut current_hash = *leaf;
        let mut current_index = self.get_leaf_node_index(index as usize);

        for sibling in proof {
            current_hash = if current_index % 2 == 0 {
                hash_pair(&current_hash, sibling)
            } else {
                hash_pair(sibling, &current_hash)
            };
            current_index = (current_index - 1) / 2;
        }

        current_hash == self.root
    }

    pub fn get_batch_status(&self, sequence_number: u64) -> Option<BatchStatus> {
        if let Some(batch) = self.processed_batches.get(&sequence_number) {
            Some(batch.status)
        } else {
            self.pending_batches
                .iter()
                .find(|b| b.sequence_number == sequence_number)
                .map(|b| b.status)
        }
    }

    pub fn finalize(&mut self) -> Result<(), ProgramError> {
        if !self.pending_batches.is_empty() {
            return Err(ProgramError::InvalidArgument);
        }
        self.metadata.is_finalized = true;
        Ok(())
    }

    fn get_leaf_node_index(&self, leaf_index: usize) -> usize {
        (1 << self.depth) - 1 + leaf_index
    }

    fn update_path_to_root(&mut self, mut node_index: usize) {
        while node_index > 0 {
            let parent_index = (node_index - 1) / 2;
            let sibling_index = if node_index % 2 == 0 {
                node_index - 1
            } else {
                node_index + 1
            };

            self.nodes[parent_index] = hash_pair(
                &self.nodes[if node_index % 2 == 0 { sibling_index } else { node_index }],
                &self.nodes[if node_index % 2 == 0 { node_index } else { sibling_index }],
            );

            node_index = parent_index;
        }
        self.root = self.nodes[0];
    }

    fn get_next_sequence_number(&self) -> u64 {
        let max_processed = self.processed_batches.keys().max().copied().unwrap_or(0);
        let max_pending = self.pending_batches
            .iter()
            .map(|b| b.sequence_number)
            .max()
            .unwrap_or(0);
        std::cmp::max(max_processed, max_pending) + 1
    }

    pub fn get_proof(&self, index: u64) -> Result<Vec<[u8; 32]>, ProgramError> {
        if index >= self.leaf_count {
            return Err(ProgramError::InvalidArgument);
        }

        let mut proof = Vec::with_capacity(self.depth);
        let mut current_index = self.get_leaf_node_index(index as usize);

        while current_index > 0 {
            let sibling_index = if current_index % 2 == 0 {
                current_index - 1
            } else {
                current_index + 1
            };
            proof.push(self.nodes[sibling_index]);
            current_index = (current_index - 1) / 2;
        }

        Ok(proof)
    }
}

fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(left);
    hasher.update(right);
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tree() -> MerkleTree {
        MerkleTree::new(
            3,
            Pubkey::new_unique(),
            1000,
            true,
        )
    }

    #[test]
    fn test_batch_operations() {
        let mut tree = create_test_tree();
        let processor = Pubkey::new_unique();
        
        // Create test leaves
        let leaves: Vec<[u8; 32]> = (0..3)
            .map(|i| {
                let mut leaf = [0u8; 32];
                leaf[0] = i as u8;
                leaf
            })
            .collect();

        // Test batch creation
        let sequence_number = tree.create_batch(
            leaves.clone(),
            processor,
            BatchType::Standard,
        ).unwrap();

        assert_eq!(
            tree.get_batch_status(sequence_number),
            Some(BatchStatus::Pending)
        );

        // Process batch
        let processed_seq = tree.process_next_batch().unwrap().unwrap();
        assert_eq!(processed_seq, sequence_number);
        assert_eq!(
            tree.get_batch_status(sequence_number),
            Some(BatchStatus::Completed)
        );

        // Verify leaves were inserted
        for (i, leaf) in leaves.iter().enumerate() {
            let proof = tree.get_proof(i as u64).unwrap();
            assert!(tree.verify(leaf, &proof, i as u64));
        }
    }

    #[test]
    fn test_priority_batches() {
        let mut tree = create_test_tree();
        let processor = Pubkey::new_unique();

        // Create standard and priority batches
        let standard_leaves = vec![[1u8; 32]];
        let priority_leaves = vec![[2u8; 32]];

        let standard_seq = tree.create_batch(
            standard_leaves,
            processor,
            BatchType::Standard,
        ).unwrap();

        let priority_seq = tree.create_batch(
            priority_leaves,
            processor,
            BatchType::Priority,
        ).unwrap();

        // Verify batch metadata
        let standard_batch = tree.pending_batches.iter()
            .find(|b| b.sequence_number == standard_seq)
            .unwrap();
        let priority_batch = tree.pending_batches.iter()
            .find(|b| b.sequence_number == priority_seq)
            .unwrap();

        assert_eq!(standard_batch.metadata.priority, 0);
        assert_eq!(priority_batch.metadata.priority, 1);
    }

    #[test]
    fn test_finalization() {
        let mut tree = create_test_tree();
        
        // Add and process a batch
        let leaves = vec![[1u8; 32]];
        let seq = tree.create_batch(
            leaves,
            Pubkey::new_unique(),
            BatchType::Standard,
        ).unwrap();
        tree.process_next_batch().unwrap();

        // Should be able to finalize when no pending batches
        assert!(tree.finalize().is_ok());
        assert!(tree.metadata.is_finalized);

        // Should not be able to add more batches after finalization
        let result = tree.create_batch(
            vec![[2u8; 32]],
            Pubkey::new_unique(),
            BatchType::Standard,
        );
        assert!(result.is_err());
    }
} 