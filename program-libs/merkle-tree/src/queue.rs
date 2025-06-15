use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::pubkey::Pubkey,
    std::collections::VecDeque,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct OperationQueue {
    queue: VecDeque<Operation>,
    max_size: usize,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct Operation {
    pub id: u64,
    pub data: Vec<u8>,
    pub processor: Pubkey,
}

impl OperationQueue {
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            max_size,
        }
    }

    pub fn enqueue(&mut self, operation: Operation) -> Result<(), &'static str> {
        if self.queue.len() >= self.max_size {
            return Err("Queue is full");
        }
        self.queue.push_back(operation);
        Ok(())
    }

    pub fn dequeue(&mut self) -> Option<Operation> {
        self.queue.pop_front()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_operations() {
        let mut queue = OperationQueue::new(2);
        let processor = Pubkey::new_unique();
        let op1 = Operation { id: 1, data: vec![1, 2, 3], processor };
        let op2 = Operation { id: 2, data: vec![4, 5, 6], processor };

        assert!(queue.enqueue(op1).is_ok());
        assert!(queue.enqueue(op2).is_ok());
        assert_eq!(queue.len(), 2);

        let op3 = Operation { id: 3, data: vec![7, 8, 9], processor };
        assert!(queue.enqueue(op3).is_err());

        let dequeued = queue.dequeue().unwrap();
        assert_eq!(dequeued.id, 1);
        assert_eq!(queue.len(), 1);
    }
} 