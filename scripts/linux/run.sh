#!/usr/bin/env bash
# Host side (Docker Desktop on Windows via Git Bash, or Docker on Linux/macOS).
# Run from the repository root:  bash scripts/linux/run.sh
# Copies the working tree (tracked + untracked, not ignored) and git history
# into a named volume, so the host's target/ and files are never written by
# the container except target/linux-out/.
set -eu
HOST=$(pwd -W 2>/dev/null || pwd)
mkdir -p target/linux-out
rm -f target/linux-out/*.txt target/linux-out/*.json
git ls-files -co --exclude-standard -z | tar --null -cf target/linux-src.tar -T -
git bundle create target/linux-src.bundle HEAD 2> /dev/null
docker build -q -t guinea-linux-verify scripts/linux > /dev/null
MSYS_NO_PATHCONV=1 docker run --rm \
  -v guinea-linux-work:/work \
  -v guinea-linux-cargo:/usr/local/cargo/registry \
  -v "$HOST/target:/host:ro" \
  -v "$HOST/target/linux-out:/out" \
  -v "$HOST/scripts/linux:/scripts:ro" \
  guinea-linux-verify bash -c 'tr -d "\r" < /scripts/verify.sh > /tmp/verify.sh && bash /tmp/verify.sh'
cat target/linux-out/summary.txt
