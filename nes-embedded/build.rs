use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
  // Copy memory.x from crate-root/nes-embedded/memory.x to target output
  // By default linker looks in crate-root, but that doesn't work with workspace setup
  let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
  File::create(out.join("memory.x"))
    .expect("output path")
    .write_all(include_bytes!("memory.x"))
    .expect("memory.x path");

  // Tell linker to look in target output
  println!("cargo:rustc-link-search={}", out.display());
  println!("cargo:rerun-if-changed=memory.x");
}