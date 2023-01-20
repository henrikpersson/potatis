#!/bin/sh
cd profile && 
  RUSTFLAGS=-g cargo run --features profile_heap --release &&
  open 'https://nnethercote.github.io/dh_view/dh_view.html'
