#!/bin/bash

set -euo pipefail
set -x

# rustup target add x86_64-apple-darwin
# rustup target add aarch64-apple-darwin
export MACOSX_DEPLOYMENT_TARGET="10.11"
cargo build --release --target=x86_64-apple-darwin
cargo build --release --target=aarch64-apple-darwin
lipo ./target/{x86_64,aarch64}-apple-darwin/release/dylibtree \
  -create -output dylibtree
COPYFILE_DISABLE=1 tar czvf dylibtree.tar.gz dylibtree
shasum -a 256 dylibtree.tar.gz dylibtree
