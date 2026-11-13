# Native Linux x86_64 verification image (dev-only; see docs/TEST_PLAN.md).
FROM rust:1.98-bookworm
RUN rustup component add rustfmt clippy \
 && rustup target add x86_64-unknown-linux-musl \
 && apt-get update \
 && apt-get install -y --no-install-recommends python3-pip musl-tools file xclip xsel xvfb wl-clipboard fonts-dejavu-core \
 && rm -rf /var/lib/apt/lists/*
ENV CARGO_TERM_COLOR=never LANG=C.UTF-8
WORKDIR /work/src
