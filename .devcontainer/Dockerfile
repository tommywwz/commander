FROM rust:latest

RUN apt-get update && apt-get install -y \
    mingw-w64 \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add x86_64-pc-windows-gnu

ENV CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc

WORKDIR /workspace