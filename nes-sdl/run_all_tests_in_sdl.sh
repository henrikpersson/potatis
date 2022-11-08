#!/bin/sh
find ../test-roms -name '*.nes' | xargs -I{} cargo run --release -- {}