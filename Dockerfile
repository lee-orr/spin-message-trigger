FROM ubuntu:latest as base

# Get Ubuntu packages
RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    pkg-config \
    libssl-dev \
    wget \
    git

RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

RUN rustup target add wasm32-unknown-unknown
RUN rustup target add wasm32-wasi

RUN <<EOF
git clone https://github.com/fermyon/spin
cd ./spin
cargo install --locked --path ./
EOF

FROM base as development
RUN cargo install cargo-watch
RUN rustup component add clippy
RUN rustup component add rustfmt
RUN rustup component add rust-analyzer
RUN cargo install mprocs
RUN cargo install --locked bacon

RUN spin plugins install --url https://github.com/itowlson/spin-pluginify/releases/download/canary/pluginify.json --yes
RUN rustup toolchain install nightly
RUN cargo install cargo-expand