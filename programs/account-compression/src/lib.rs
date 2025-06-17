use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint,
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
        clock::Clock,
        sysvar::Sysvar,
    },
    std::collections::HashMap,
};

// Declare the program's entrypoint
entrypoint!(process_instruction);

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum AccountCompressionInstruction {
    InitializeCompression {
        max_depth: u32,
        max_buffer_size: u32,
    },
    CompressAccount {
        account_type: AccountType,
        compression_config: CompressionConfig,
    },
    DecompressAccount {
        account_id: Pubkey,
    },
    UpdateCompressionParams {
        new_config: CompressionConfig,
    },
    ValidateCompression {
        account_id: Pubkey,
        expected_hash: [u8; 32],
    },
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct CompressionConfig {
    pub algorithm: CompressionAlgorithm,
    pub level: u8,
    pub chunk_size: u32,
    pub concurrent_compression: bool,
    pub verify_compression: bool,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub enum CompressionAlgorithm {
    Lz4,
    Snappy,
    Zstd,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum AccountType {
    User,
    Token,
    NFT,
    Program,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CompressedAccountState {
    pub is_compressed: bool,
    pub original_size: u64,
    pub compressed_size: u64,
    pub compression_algorithm: CompressionAlgorithm,
    pub last_modified: i64,
    pub compression_stats: CompressionStats,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CompressionStats {
    pub total_compressions: u64,
    pub total_decompressions: u64,
    pub average_compression_ratio: f64,
    pub best_compression_ratio: f64,
    pub total_bytes_saved: u64,
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = AccountCompressionInstruction::try_from_slice(instruction_data)?;
    let account_info_iter = &mut accounts.iter();

    match instruction {
        AccountCompressionInstruction::InitializeCompression { max_depth, max_buffer_size } => {
            msg!("Instruction: InitializeCompression");
            process_initialize_compression(program_id, account_info_iter, max_depth, max_buffer_size)
        }
        AccountCompressionInstruction::CompressAccount { account_type, compression_config } => {
            msg!("Instruction: CompressAccount");
            process_compress_account(program_id, account_info_iter, account_type, compression_config)
        }
        AccountCompressionInstruction::DecompressAccount { account_id } => {
            msg!("Instruction: DecompressAccount");
            process_decompress_account(program_id, account_info_iter, account_id)
        }
        AccountCompressionInstruction::UpdateCompressionParams { new_config } => {
            msg!("Instruction: UpdateCompressionParams");
            process_update_compression_params(program_id, account_info_iter, new_config)
        }
        AccountCompressionInstruction::ValidateCompression { account_id, expected_hash } => {
            msg!("Instruction: ValidateCompression");
            process_validate_compression(program_id, account_info_iter, account_id, expected_hash)
        }
    }
}

fn process_initialize_compression(
    program_id: &Pubkey,
    account_info_iter: &mut std::slice::Iter<AccountInfo>,
    max_depth: u32,
    max_buffer_size: u32,
) -> ProgramResult {
    let admin_account = next_account_info(account_info_iter)?;
    let state_account = next_account_info(account_info_iter)?;

    // Verify admin account
    if !admin_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Initialize compression state
    let compression_state = CompressedAccountState {
        is_compressed: false,
        original_size: 0,
        compressed_size: 0,
        compression_algorithm: CompressionAlgorithm::Lz4,
        last_modified: Clock::get()?.unix_timestamp,
        compression_stats: CompressionStats {
            total_compressions: 0,
            total_decompressions: 0,
            average_compression_ratio: 1.0,
            best_compression_ratio: 1.0,
            total_bytes_saved: 0,
        },
    };

    compression_state.serialize(&mut *state_account.try_borrow_mut_data()?)?;
    Ok(())
}

fn process_compress_account(
    program_id: &Pubkey,
    account_info_iter: &mut std::slice::Iter<AccountInfo>,
    account_type: AccountType,
    compression_config: CompressionConfig,
) -> ProgramResult {
    let account_to_compress = next_account_info(account_info_iter)?;
    let state_account = next_account_info(account_info_iter)?;

    // Verify account ownership
    if account_to_compress.owner != program_id {
        return Err(ProgramError::InvalidAccountData);
    }

    // Read current state
    let mut compression_state = CompressedAccountState::try_from_slice(&state_account.try_borrow_data()?)?;

    // Perform compression based on account type and config
    let data = account_to_compress.try_borrow_data()?;
    let original_size = data.len() as u64;
    
    let compressed_data = match compression_config.algorithm {
        CompressionAlgorithm::Lz4 => compress_lz4(&data, compression_config.level)?,
        CompressionAlgorithm::Snappy => compress_snappy(&data)?,
        CompressionAlgorithm::Zstd => compress_zstd(&data, compression_config.level)?,
    };

    // Update compression stats
    let compressed_size = compressed_data.len() as u64;
    let compression_ratio = original_size as f64 / compressed_size as f64;
    
    compression_state.compression_stats.total_compressions += 1;
    compression_state.compression_stats.average_compression_ratio = 
        (compression_state.compression_stats.average_compression_ratio * (compression_state.compression_stats.total_compressions - 1) as f64
        + compression_ratio) / compression_state.compression_stats.total_compressions as f64;
    
    if compression_ratio > compression_state.compression_stats.best_compression_ratio {
        compression_state.compression_stats.best_compression_ratio = compression_ratio;
    }

    compression_state.compression_stats.total_bytes_saved += original_size - compressed_size;
    compression_state.last_modified = Clock::get()?.unix_timestamp;
    
    // Save compressed data and updated state
    compression_state.serialize(&mut *state_account.try_borrow_mut_data()?)?;

    Ok(())
}

fn process_decompress_account(
    program_id: &Pubkey,
    account_info_iter: &mut std::slice::Iter<AccountInfo>,
    account_id: Pubkey,
) -> ProgramResult {
    let account_to_decompress = next_account_info(account_info_iter)?;
    let state_account = next_account_info(account_info_iter)?;

    // Verify account
    if account_to_decompress.key != &account_id {
        return Err(ProgramError::InvalidArgument);
    }

    // Read compression state
    let mut compression_state = CompressedAccountState::try_from_slice(&state_account.try_borrow_data()?)?;

    if !compression_state.is_compressed {
        return Err(ProgramError::InvalidAccountData);
    }

    // Perform decompression
    let compressed_data = account_to_decompress.try_borrow_data()?;
    let decompressed_data = match compression_state.compression_algorithm {
        CompressionAlgorithm::Lz4 => decompress_lz4(&compressed_data, compression_state.original_size as usize)?,
        CompressionAlgorithm::Snappy => decompress_snappy(&compressed_data, compression_state.original_size as usize)?,
        CompressionAlgorithm::Zstd => decompress_zstd(&compressed_data, compression_state.original_size as usize)?,
    };

    // Update stats
    compression_state.compression_stats.total_decompressions += 1;
    compression_state.last_modified = Clock::get()?.unix_timestamp;
    compression_state.is_compressed = false;

    // Save state
    compression_state.serialize(&mut *state_account.try_borrow_mut_data()?)?;

    Ok(())
}

fn process_update_compression_params(
    program_id: &Pubkey,
    account_info_iter: &mut std::slice::Iter<AccountInfo>,
    new_config: CompressionConfig,
) -> ProgramResult {
    let admin_account = next_account_info(account_info_iter)?;
    let config_account = next_account_info(account_info_iter)?;

    // Verify admin
    if !admin_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Update configuration
    new_config.serialize(&mut *config_account.try_borrow_mut_data()?)?;

    Ok(())
}

fn process_validate_compression(
    program_id: &Pubkey,
    account_info_iter: &mut std::slice::Iter<AccountInfo>,
    account_id: Pubkey,
    expected_hash: [u8; 32],
) -> ProgramResult {
    let account_to_validate = next_account_info(account_info_iter)?;
    let state_account = next_account_info(account_info_iter)?;

    // Verify account
    if account_to_validate.key != &account_id {
        return Err(ProgramError::InvalidArgument);
    }

    // Read state and verify hash
    let compression_state = CompressedAccountState::try_from_slice(&state_account.try_borrow_data()?)?;
    
    if !compression_state.is_compressed {
        return Err(ProgramError::InvalidAccountData);
    }

    // Calculate hash of compressed data
    let data = account_to_validate.try_borrow_data()?;
    let mut hasher = sha2::Sha256::new();
    hasher.update(&data);
    let actual_hash = hasher.finalize();

    if actual_hash.as_slice() != expected_hash {
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(())
}

// Helper functions for compression algorithms
fn compress_lz4(data: &[u8], level: u8) -> Result<Vec<u8>, ProgramError> {
    let mut encoder = lz4_flex::frame::FrameEncoder::new(Vec::new());
    std::io::Write::write_all(&mut encoder, data).map_err(|_| ProgramError::InvalidAccountData)?;
    encoder.finish().map_err(|_| ProgramError::InvalidAccountData)
}

fn decompress_lz4(compressed: &[u8], original_size: usize) -> Result<Vec<u8>, ProgramError> {
    let mut decoder = lz4_flex::frame::FrameDecoder::new(compressed);
    let mut decompressed = Vec::with_capacity(original_size);
    std::io::copy(&mut decoder, &mut decompressed).map_err(|_| ProgramError::InvalidAccountData)?;
    Ok(decompressed)
}

fn compress_snappy(data: &[u8]) -> Result<Vec<u8>, ProgramError> {
    snap::raw::Encoder::new()
        .compress_vec(data)
        .map_err(|_| ProgramError::InvalidAccountData)
}

fn decompress_snappy(compressed: &[u8], original_size: usize) -> Result<Vec<u8>, ProgramError> {
    snap::raw::Decoder::new()
        .decompress_vec(compressed)
        .map_err(|_| ProgramError::InvalidAccountData)
}

fn compress_zstd(data: &[u8], level: u8) -> Result<Vec<u8>, ProgramError> {
    zstd::encode_all(data, level as i32)
        .map_err(|_| ProgramError::InvalidAccountData)
}

fn decompress_zstd(compressed: &[u8], original_size: usize) -> Result<Vec<u8>, ProgramError> {
    zstd::decode_all(compressed)
        .map_err(|_| ProgramError::InvalidAccountData)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::clock::Epoch;

    // Helper function to create test accounts
    fn create_test_account(owner: &Pubkey, data_size: usize) -> AccountInfo {
        AccountInfo::new(
            &Pubkey::new_unique(),
            false,
            true,
            &mut 0,
            &mut vec![0; data_size],
            owner,
            false,
            Epoch::default(),
        )
    }

    #[test]
    fn test_initialize_compression() {
        let program_id = Pubkey::new_unique();
        let admin = create_test_account(&program_id, 0);
        let mut state_data = vec![0; 1000];
        let state = AccountInfo::new(
            &Pubkey::new_unique(),
            false,
            true,
            &mut 0,
            &mut state_data,
            &program_id,
            false,
            Epoch::default(),
        );

        let accounts = vec![admin, state];
        let result = process_initialize_compression(
            &program_id,
            &mut accounts.iter(),
            32,
            1024,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_compression_workflow() {
        let program_id = Pubkey::new_unique();
        let test_data = vec![1, 2, 3, 4, 5];
        let account = create_test_account(&program_id, test_data.len());
        let mut state_data = vec![0; 1000];
        let state = AccountInfo::new(
            &Pubkey::new_unique(),
            false,
            true,
            &mut 0,
            &mut state_data,
            &program_id,
            false,
            Epoch::default(),
        );

        let config = CompressionConfig {
            algorithm: CompressionAlgorithm::Lz4,
            level: 1,
            chunk_size: 1024,
            concurrent_compression: false,
            verify_compression: true,
        };

        let accounts = vec![account.clone(), state.clone()];
        let result = process_compress_account(
            &program_id,
            &mut accounts.iter(),
            AccountType::User,
            config,
        );

        assert!(result.is_ok());
    }
} 