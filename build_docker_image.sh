#!/bin/bash
cargo build --release

TEMP_DIR=$(mktemp -d)
cp target/release/minio-operator "$TEMP_DIR"

# Download mc
wget -O "$TEMP_DIR/mc" https://dl.min.io/client/mc/release/linux-amd64/mc
chmod +x "$TEMP_DIR/mc" 

docker build -f Dockerfile "$TEMP_DIR" -t pierre42100/minio_operator

rm -r $TEMP_DIR

