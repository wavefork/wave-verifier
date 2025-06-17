use solana_program::{
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

#[derive(Debug)]
pub enum WaveEvent {
    FlowRegistered {
        flow_id: u64,
        merkle_root: Option<[u8; 32]>,
        circuit_hash: [u8; 32],
    },
    FlowExecuted {
        flow_id: u64,
        nullifier: [u8; 32],
    },
    ProofRejected {
        flow_id: u64,
        reason: String,
    },
    NullifierUsed {
        nullifier: [u8; 32],
        flow_id: u64,
        timestamp: i64,
    },
    RootUpdated {
        flow_id: u64,
        new_root: [u8; 32],
    },
    FlowTriggered {
        flow_id: u64,
        target_program: Pubkey,
    },
}

#[cfg(test)]
pub struct EventLogger {
    pub events: Vec<WaveEvent>,
}

#[cfg(test)]
impl EventLogger {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn log(&mut self, event: WaveEvent) {
        self.events.push(event);
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}

impl WaveEvent {
    pub fn emit(&self) {
        match self {
            Self::FlowRegistered { flow_id, merkle_root, circuit_hash } => {
                msg!("Event: FlowRegistered");
                msg!("  flow_id: {}", flow_id);
                msg!("  merkle_root: {:?}", merkle_root);
                msg!("  circuit_hash: {:?}", circuit_hash);
            }
            Self::FlowExecuted { flow_id, nullifier } => {
                msg!("Event: FlowExecuted");
                msg!("  flow_id: {}", flow_id);
                msg!("  nullifier: {:?}", nullifier);
            }
            Self::ProofRejected { flow_id, reason } => {
                msg!("Event: ProofRejected");
                msg!("  flow_id: {}", flow_id);
                msg!("  reason: {}", reason);
            }
            Self::NullifierUsed { nullifier, flow_id, timestamp } => {
                msg!("Event: NullifierUsed");
                msg!("  nullifier: {:?}", nullifier);
                msg!("  flow_id: {}", flow_id);
                msg!("  timestamp: {}", timestamp);
            }
            Self::RootUpdated { flow_id, new_root } => {
                msg!("Event: RootUpdated");
                msg!("  flow_id: {}", flow_id);
                msg!("  new_root: {:?}", new_root);
            }
            Self::FlowTriggered { flow_id, target_program } => {
                msg!("Event: FlowTriggered");
                msg!("  flow_id: {}", flow_id);
                msg!("  target_program: {}", target_program);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::test_data::*;

    #[test]
    fn test_event_logger() {
        let mut logger = EventLogger::new();
        
        let event = WaveEvent::FlowRegistered {
            flow_id: FLOW_ID_1,
            merkle_root: Some(MERKLE_ROOT_1),
            circuit_hash: CIRCUIT_HASH_1,
        };
        
        logger.log(event);
        assert_eq!(logger.events.len(), 1);
        
        logger.clear();
        assert_eq!(logger.events.len(), 0);
    }

    #[test]
    fn test_event_emission() {
        let event = WaveEvent::FlowExecuted {
            flow_id: FLOW_ID_1,
            nullifier: NULLIFIER_1,
        };
        
        // This will print to program logs
        event.emit();
    }
} 