#!/bin/env bash

# Obviously I wouldn't include this file in any actual PR, but it's here to document how I've been emulating the implementation.

set -e

if [ -n "$RISCV_DEBUG" ] ; then
    RUSTFLAGS="$RUSTFLAGS -C debuginfo=2"
fi

# rustup install 1.81-x86_64-unknown-linux-gnu
# rustup target add riscv64gc-unknown-linux-gnu
RUSTFLAGS="$RUSTFLAGS -C linker=riscv64-linux-gnu-gcc -C target-feature=+crt-static" cargo build --target=riscv64gc-unknown-linux-gnu --example riscv --release --no-default-features --features "singlepass"

if [ -n "$RISCV_DEBUG" ] ; then
    qemu-riscv64 -g 1234 ./target/riscv64gc-unknown-linux-gnu/release/examples/riscv &
    PID=`echo $!`
    printf "Run \n\tgdb-multiarch ./target/riscv64gc-unknown-linux-gnu/release/examples/riscv\nand then \n\ttarget remote localhost:1234\nto debug"
    read -p "Press enter to exit"
    kill -9 "$PID"
else
    qemu-riscv64 ./target/riscv64gc-unknown-linux-gnu/release/examples/riscv
fi
