#!/bin/sh

echo "Building your Event Stream Processor..."
# Compile your Rust project. Adjust 'release' if you prefer debug builds.
cargo build --release

if [ $? -ne 0 ]; then
    echo "Rust build failed! Aborting."
    exit 1
fi

PROGRAM_NAME="Eventor"

echo "Running your Event Stream Processor in the foreground (Ctrl+C to stop)..."
# Run the compiled program directly
./target/release/"$PROGRAM_NAME"