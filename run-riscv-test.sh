#!/bin/env bash

# Obviously I wouldn't include this file in any actual PR, but it's here to document how I've been emulating the implementation.
RUSTFLAGS="-C target-feature=+crt-static" cargo build --target=riscv64gc-unknown-linux-gnu --example riscv --release --no-default-features --features "singlepass" && qemu-riscv64 ./target/riscv64gc-unknown-linux-gnu/release/examples/riscv
