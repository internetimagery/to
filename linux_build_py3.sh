#!/bin/bash
cargo build --release --features python3
mv target/release/libto.so `git rev-parse --show-toplevel`/to/_internal.so

