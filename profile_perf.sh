#!/bin/sh
cd profile && 
  RUSTFLAGS=-g cargo build --release --features profile_cpu_no_std && 
  samply record ../target/release/profile
