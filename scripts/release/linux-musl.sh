#!/usr/bin/env bash
# Inside the Linux verification container (scripts/linux/verify.sh calls it):
# build static x86_64-unknown-linux-musl release binaries and check they load
# without glibc (release plan R0.3). Output: target/x86_64-unknown-linux-musl/release.
set -eu
TARGET=x86_64-unknown-linux-musl
cargo build --workspace --release --locked --target "$TARGET"
DIR="target/$TARGET/release"
for bin in guinea guineanote-agent guineanote-mcp; do
  file "$DIR/$bin"
  # ldd exits non-zero for static executables; record its message.
  ldd "$DIR/$bin" 2>&1 || true
  "$DIR/$bin" --version
done
file "$DIR/guinea" | grep -Eq "static-pie linked|statically linked" || { echo "guinea is not static"; exit 1; }
