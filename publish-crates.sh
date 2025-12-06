#!/bin/bash

set -e

CRATES=(
    "crates/rohas-parser"
    "crates/rohas-codegen"
    "crates/rohas-runtime"
    "crates/rohas-telemetry"
    "crates/rohas-engine"
    "crates/rohas-cron"
    "crates/rohas-dev-server"
    "crates/rohas-cli"
    "crates/rohas-adapters/adapter-memory"
    "crates/rohas-adapters/adapter-nats"
    "crates/rohas-adapters/adapter-kafka"
    "crates/rohas-adapters/adapter-rabbitmq"
    "crates/rohas-adapters/adapter-aws"
    "crates/rohas-adapters/adapter-rocksdb"
)

DELAY_SECONDS=720  # 12 minutes

# Get the last crate for comparison
LAST_CRATE="${CRATES[${#CRATES[@]}-1]}"

echo "Publishing remaining crates to crates.io..."
echo "Delay between crates: ${DELAY_SECONDS} seconds (~12 minutes)"
echo ""

for crate in "${CRATES[@]}"; do
    echo "=========================================="
    echo "Publishing: $crate"
    echo "=========================================="
    
    if cargo publish --manifest-path "$crate/Cargo.toml" --allow-dirty; then
        echo "✓ Successfully published $crate"
    else
        echo "✗ Failed to publish $crate"
        echo "Waiting before next attempt..."
        sleep "$DELAY_SECONDS"
        continue
    fi
    
    # Don't wait after the last crate
    if [ "$crate" != "$LAST_CRATE" ]; then
        echo "Waiting ${DELAY_SECONDS} seconds before next publish..."
        sleep "$DELAY_SECONDS"
    fi
done

echo ""
echo "=========================================="
echo "All crates published!"
echo "=========================================="
