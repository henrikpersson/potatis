#!/bin/sh
cargo build --release --bin nes-cloud-instance && cargo test --release -- --test-threads=1 --nocapture