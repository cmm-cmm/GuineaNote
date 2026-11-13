#!/usr/bin/env bash
# Host side: check the packed Linux archive the way a user gets it. Unpacks it
# in the verification image, checks static linkage, runs the PTY smoke probe
# with the unpacked binary (probe scripts from the tag), then runs --version
# on several distributions.
#   bash scripts/release/linux-check.sh v0.1.0 target/dist/v0.1.0
set -eu
TAG="$1"
DIST="$2"
VERSION="${TAG#v}"
NAME="guinea-$VERSION-x86_64-unknown-linux-musl"
HOST=$(pwd -W 2>/dev/null || pwd)
[ -f "$DIST/$NAME.tar.gz" ] || { echo "missing $DIST/$NAME.tar.gz"; exit 1; }
REPORT="$DIST/linux-check.txt"
MSYS_NO_PATHCONV=1 docker run --rm \
  -v "$HOST/target/release-$TAG.bundle:/src.bundle:ro" \
  -v "$HOST/$DIST:/dist:ro" \
  -e TAG="$TAG" -e NAME="$NAME" \
  guinea-linux-verify bash -c '
    set -eu
    cd /tmp && (cd /dist && sha256sum --ignore-missing -c SHA256SUMS)
    tar -xzf "/dist/$NAME.tar.gz" -C /tmp
    ls -l "/tmp/$NAME"
    file "/tmp/$NAME/guinea" | tee /dev/stderr | grep -Eq "static-pie linked|statically linked"
    for bin in guinea guineanote-agent guineanote-mcp; do "/tmp/$NAME/$bin" --version; done
    git clone -q /src.bundle /tmp/src && cd /tmp/src && git -c advice.detachedHead=false checkout -q "$TAG"
    python3 -m pip install -q --break-system-packages --target target/pty-tools psutil==7.2.2 pyte==0.8.2
    python3 -X utf8 scripts/pty_smoke.py --binary "/tmp/$NAME/guinea"' 2>&1 | tee "$REPORT"
for image in debian:11 debian:12 ubuntu:22.04 alpine:3.20; do
  echo "== $image" | tee -a "$REPORT"
  MSYS_NO_PATHCONV=1 docker run --rm -v "$HOST/$DIST:/dist:ro" "$image" sh -c \
    "cd /tmp && tar -xzf /dist/$NAME.tar.gz && ./$NAME/guinea --version" 2>&1 | tee -a "$REPORT"
done
