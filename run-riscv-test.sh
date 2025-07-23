#!/bin/env bash

# Obviously I wouldn't include this file in any actual PR, but it's here to document how I've been emulating the implementation.
# rustup install 1.81-x86_64-unknown-linux-gnu
# rustup target add riscv64gc-unknown-linux-gnu
RUSTFLAGS="-C linker=riscv64-linux-gnu-gcc -C target-feature=+crt-static" cargo build --target=riscv64gc-unknown-linux-gnu --example riscv --release --no-default-features --features "singlepass" && qemu-riscv64 ./target/riscv64gc-unknown-linux-gnu/release/examples/riscv
