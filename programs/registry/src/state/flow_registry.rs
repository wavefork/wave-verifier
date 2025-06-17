use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
};

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq)]
pub struct FlowRegistry {
    /// The authority that can update this flow's settings
    pub authority: Pubkey,
    /// The flow ID
    pub flow_id: u64,
    /// Optional Merkle root for membership verification
    pub merkle_root: Option<[u8; 32]>,
    /// Hash of the circuit used for this flow
    pub circuit_hash: [u8; 32],
    /// Whether the flow is currently enabled
    pub is_enabled: bool,
    /// Optional program ID to call after successful verification
    pub callback_program_id: Option<Pubkey>,
}

impl FlowRegistry {
    pub const SIZE: usize = 32 + 8 + 33 + 32 + 1 + 33;

    pub fn new(
        authority: Pubkey,
        flow_id: u64,
        merkle_root: Option<[u8; 32]>,
        circuit_hash: [u8; 32],
        callback_program_id: Option<Pubkey>,
    ) -> Self {
        Self {
            authority,
            flow_id,
            merkle_root,
            circuit_hash,
            is_enabled: true,
            callback_program_id,
        }
    }

    pub fn save(&self, account: &AccountInfo) -> Result<(), ProgramError> {
        let data = self.try_to_vec()?;
        let mut account_data = account.try_borrow_mut_data()?;
        account_data[..data.len()].copy_from_slice(&data);
        Ok(())
    }

    pub fn load(account: &AccountInfo) -> Result<Self, ProgramError> {
        let data = account.try_borrow_data()?;
        let registry = Self::try_from_slice(&data)?;
        Ok(registry)
    }
}

#[cfg(test)]
pub struct RegistryManager {
    pub registries: Vec<FlowRegistry>,
}

#[cfg(test)]
impl RegistryManager {
    pub fn new() -> Self {
        Self {
            registries: Vec::new(),
        }
    }

    pub fn add_registry(&mut self, registry: FlowRegistry) {
        self.registries.push(registry);
    }

    pub fn get_by_id(&self, flow_id: u64) -> Option<&FlowRegistry> {
        self.registries.iter().find(|r| r.flow_id == flow_id)
    }

    pub fn update_root(&mut self, flow_id: u64, new_root: [u8; 32]) -> Result<(), ProgramError> {
        if let Some(registry) = self.registries.iter_mut().find(|r| r.flow_id == flow_id) {
            registry.merkle_root = Some(new_root);
            Ok(())
        } else {
            Err(ProgramError::InvalidAccountData)
        }
    }

    pub fn set_enabled(&mut self, flow_id: u64, enabled: bool) -> Result<(), ProgramError> {
        if let Some(registry) = self.registries.iter_mut().find(|r| r.flow_id == flow_id) {
            registry.is_enabled = enabled;
            Ok(())
        } else {
            Err(ProgramError::InvalidAccountData)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::test_data::*;

    #[test]
    fn test_flow_registry() {
        let authority = Pubkey::new_unique();
        let registry = FlowRegistry::new(
            authority,
            FLOW_ID_1,
            Some(MERKLE_ROOT_1),
            CIRCUIT_HASH_1,
            None,
        );

        assert_eq!(registry.authority, authority);
        assert_eq!(registry.flow_id, FLOW_ID_1);
        assert_eq!(registry.merkle_root, Some(MERKLE_ROOT_1));
        assert_eq!(registry.circuit_hash, CIRCUIT_HASH_1);
        assert!(registry.is_enabled);
    }

    #[test]
    fn test_registry_manager() {
        let mut manager = RegistryManager::new();
        
        let registry1 = FlowRegistry::new(
            Pubkey::new_unique(),
            FLOW_ID_1,
            Some(MERKLE_ROOT_1),
            CIRCUIT_HASH_1,
            None,
        );
        manager.add_registry(registry1);

        let registry2 = FlowRegistry::new(
            Pubkey::new_unique(),
            FLOW_ID_2,
            Some(MERKLE_ROOT_2),
            CIRCUIT_HASH_2,
            None,
        );
        manager.add_registry(registry2);

        let found = manager.get_by_id(FLOW_ID_1).unwrap();
        assert_eq!(found.flow_id, FLOW_ID_1);

        manager.update_root(FLOW_ID_1, MERKLE_ROOT_3).unwrap();
        let updated = manager.get_by_id(FLOW_ID_1).unwrap();
        assert_eq!(updated.merkle_root, Some(MERKLE_ROOT_3));

        manager.set_enabled(FLOW_ID_1, false).unwrap();
        let disabled = manager.get_by_id(FLOW_ID_1).unwrap();
        assert!(!disabled.is_enabled);
    }
} 