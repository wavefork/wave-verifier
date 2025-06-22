#!/bin/bash

echo "Building programs..."
./scripts/build.sh

echo "Starting local deployment..."

# Deploy Account Compression Program
echo "Deploying Account Compression Program..."
solana program deploy \
    ./target/deploy/account_compression.so \
    --url localhost \
    --keypair ~/.config/solana/id.json \
    --program-id ./target/deploy/account_compression-keypair.json

# Store program ID for future use
COMPRESSION_PROGRAM_ID=$(solana-keygen pubkey ./target/deploy/account_compression-keypair.json)
echo "Account Compression Program deployed with ID: $COMPRESSION_PROGRAM_ID"

# Initialize the program with default settings
echo "Initializing Account Compression Program..."
solana program call \
    --url localhost \
    --keypair ~/.config/solana/id.json \
    $COMPRESSION_PROGRAM_ID initialize \
    32 1024 # max_depth and max_buffer_size

echo "Local deployment completed successfully!"
echo "Program IDs:"
echo "Account Compression: $COMPRESSION_PROGRAM_ID" 