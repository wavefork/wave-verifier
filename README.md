# WaveFork Verifier

A Solana program suite for efficient on-chain data compression and verification. This project includes multiple programs for handling account data compression, registry management, and state verification.

## Programs

### Account Compression Program
- Efficient on-chain data compression using multiple algorithms (LZ4, Snappy, Zstd)
- Configurable compression parameters
- Concurrent compression support
- Data integrity verification
- Compression queue management

## Prerequisites

- Rust 1.70.0 or later
- Solana CLI tools v1.16 or later
- [Anchor](https://www.anchor-lang.com/) framework
- Node.js v16+ (for deployment scripts)

## Installation

1. Install Rust:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. Install Solana CLI tools:
```bash
sh -c "$(curl -sSfL https://release.solana.com/v1.16.0/install)"
```

3. Install project dependencies:
```bash
cargo build
```

## Development Setup

1. Configure your Solana cluster:
```bash
solana config set --url localhost
```

2. Start a local validator:
```bash
solana-test-validator
```

3. Build the programs:
```bash
./scripts/build.sh
```

## Testing

### Run Integration Tests
```bash
cargo test --test integration_tests
```

### Run Performance Tests
```bash
cargo test --test compression_performance_tests
```

### Run Security Tests
```bash
cargo test --test security_tests
```

### Run Benchmarks
```bash
cargo bench
```

## Program Deployment

1. Build the programs:
```bash
./scripts/build.sh
```

2. Deploy to localnet:
```bash
./scripts/deploy-local.sh
```

3. Deploy to devnet:
```bash
./scripts/deploy-devnet.sh
```

4. Deploy to mainnet:
```bash
./scripts/deploy-mainnet.sh
```

## Program Architecture

### Account Compression Program
- `src/lib.rs`: Main program entrypoint and instruction processing
- `src/error.rs`: Custom error types
- `src/state.rs`: Program state management
- `src/processor.rs`: Instruction processing logic

### Testing Structure
- `tests/integration/`: Integration tests
  - `src/lib.rs`: Main test suite
  - `src/compression_performance_tests.rs`: Performance benchmarks
  - `src/security_tests.rs`: Security and edge cases

## Scripts

- `scripts/build.sh`: Build all programs
- `scripts/clean.sh`: Clean build artifacts
- `scripts/test.sh`: Run all tests
- `scripts/format.sh`: Format code using rustfmt
- `scripts/lint.sh`: Run clippy lints
- `scripts/deploy-local.sh`: Deploy to local validator
- `scripts/deploy-devnet.sh`: Deploy to devnet
- `scripts/deploy-mainnet.sh`: Deploy to mainnet

## Configuration

### Program Settings
Program configurations can be modified in their respective `src/lib.rs` files:

```rust
pub const MAX_COMPRESSION_RATIO: f64 = 10.0;
pub const MIN_CHUNK_SIZE: usize = 1024;
pub const MAX_CHUNK_SIZE: usize = 32768;
pub const MAX_CONCURRENT_COMPRESSIONS: u32 = 8;
```

### Compression Algorithms
Available compression algorithms:
- LZ4: Fast compression/decompression
- Snappy: Optimized for speed
- Zstd: Better compression ratios

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Run tests (`cargo test`)
4. Run clippy (`cargo clippy`)
5. Format code (`cargo fmt`)
6. Commit changes (`git commit -m 'Add amazing feature'`)
7. Push to branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

## Security

For security concerns, please open an issue or contact the maintainers directly.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Solana Labs for the blockchain platform
- The Rust community for excellent compression libraries
- Contributors and maintainers