#!/usr/bin/env bash
# Host side (Docker Desktop via Git Bash, or Docker on Linux): build the static
# Linux release binaries from a tag in a clean clone inside the verification
# image. Nothing from the working tree is used except the tag's history.
#   bash scripts/release/linux-build.sh v0.1.0    -> target/release-linux/v0.1.0/
set -eu
TAG="$1"
HOST=$(pwd -W 2>/dev/null || pwd)
OUT="target/release-linux/$TAG"
[ ! -e "$OUT" ] || { echo "$OUT exists"; exit 1; }
mkdir -p "$OUT"
git bundle create "target/release-$TAG.bundle" "$TAG" 2> /dev/null
docker build -q -t guinea-linux-verify scripts/linux > /dev/null
MSYS_NO_PATHCONV=1 docker run --rm \
  -v guinea-linux-cargo:/usr/local/cargo/registry \
  -v "$HOST/target/release-$TAG.bundle:/src.bundle:ro" \
  -v "$HOST/$OUT:/out" \
  -e TAG="$TAG" \
  guinea-linux-verify bash -c '
    set -eu
    git clone -q /src.bundle /tmp/src
    cd /tmp/src
    git -c advice.detachedHead=false checkout -q "$TAG"
    bash scripts/release/linux-musl.sh > /out/build.txt 2>&1 || { cat /out/build.txt; exit 1; }
    cp target/x86_64-unknown-linux-musl/release/guinea \
       target/x86_64-unknown-linux-musl/release/guineanote-agent \
       target/x86_64-unknown-linux-musl/release/guineanote-mcp /out/
    rustc --version > /out/rustc.txt
    git rev-parse HEAD > /out/commit.txt
    cd /out && sha256sum guinea guineanote-agent guineanote-mcp > binary.sha256'
cat "$OUT/binary.sha256"
