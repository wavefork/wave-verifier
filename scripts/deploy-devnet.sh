#!/bin/bash

echo "Building programs..."
./scripts/build.sh

echo "Starting devnet deployment..."

# Check devnet SOL balance
BALANCE=$(solana balance --url devnet)
MIN_BALANCE=2  # SOL
if (( $(echo "$BALANCE < $MIN_BALANCE" | bc -l) )); then
    echo "Insufficient devnet SOL. Airdropping 2 SOL..."
    solana airdrop 2 --url devnet
    sleep 2  # Wait for airdrop to confirm
fi

# Deploy Account Compression Program
echo "Deploying Account Compression Program..."
solana program deploy \
    ./target/deploy/account_compression.so \
    --url devnet \
    --keypair ~/.config/solana/id.json \
    --program-id ./target/deploy/account_compression-keypair.json

# Store program ID for future use
COMPRESSION_PROGRAM_ID=$(solana-keygen pubkey ./target/deploy/account_compression-keypair.json)
echo "Account Compression Program deployed with ID: $COMPRESSION_PROGRAM_ID"

# Initialize the program with default settings
echo "Initializing Account Compression Program..."
solana program call \
    --url devnet \
    --keypair ~/.config/solana/id.json \
    $COMPRESSION_PROGRAM_ID initialize \
    32 1024 # max_depth and max_buffer_size

# Save program IDs to a config file
echo "Saving program IDs to config..."
cat > ./target/deploy/program-ids.json << EOF
{
    "compression_program": "$COMPRESSION_PROGRAM_ID",
    "network": "devnet",
    "deployment_timestamp": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
}
EOF

echo "Devnet deployment completed successfully!"
echo "Program IDs:"
echo "Account Compression: $COMPRESSION_PROGRAM_ID"
echo "Configuration saved to ./target/deploy/program-ids.json" 