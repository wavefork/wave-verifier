use solana_program::program_error::ProgramError;
use std::io::{self, Write};

pub fn compress_lz4(data: &[u8]) -> Result<Vec<u8>, ProgramError> {
    let mut encoder = lz4_flex::frame::FrameEncoder::new(Vec::new());
    encoder.write_all(data).map_err(|_| ProgramError::InvalidArgument)?;
    encoder.finish().map_err(|_| ProgramError::InvalidArgument)
}

pub fn decompress_lz4(compressed: &[u8], original_size: usize) -> Result<Vec<u8>, ProgramError> {
    let mut decoder = lz4_flex::frame::FrameDecoder::new(compressed);
    let mut decompressed = Vec::with_capacity(original_size);
    io::copy(&mut decoder, &mut decompressed)
        .map_err(|_| ProgramError::InvalidArgument)?;
    Ok(decompressed)
}

pub fn compress_snappy(data: &[u8]) -> Result<Vec<u8>, ProgramError> {
    snap::raw::Encoder::new()
        .compress_vec(data)
        .map_err(|_| ProgramError::InvalidArgument)
}

pub fn decompress_snappy(compressed: &[u8], original_size: usize) -> Result<Vec<u8>, ProgramError> {
    snap::raw::Decoder::new()
        .decompress_vec(compressed)
        .map_err(|_| ProgramError::InvalidArgument)
}

pub fn compress_zstd(data: &[u8]) -> Result<Vec<u8>, ProgramError> {
    zstd::encode_all(data, 0)
        .map_err(|_| ProgramError::InvalidArgument)
}

pub fn decompress_zstd(compressed: &[u8], original_size: usize) -> Result<Vec<u8>, ProgramError> {
    zstd::decode_all(compressed)
        .map_err(|_| ProgramError::InvalidArgument)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lz4_compression() {
        let data = b"Hello, LZ4!";
        let compressed = compress_lz4(data).unwrap();
        let decompressed = decompress_lz4(&compressed, data.len()).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_snappy_compression() {
        let data = b"Hello, Snappy!";
        let compressed = compress_snappy(data).unwrap();
        let decompressed = decompress_snappy(&compressed, data.len()).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_zstd_compression() {
        let data = b"Hello, Zstd!";
        let compressed = compress_zstd(data).unwrap();
        let decompressed = decompress_zstd(&compressed, data.len()).unwrap();
        assert_eq!(decompressed, data);
    }
} 