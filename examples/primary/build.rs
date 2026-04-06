use boards::bsp::Board;
use boards::memory::BoardMemory;
use boards::memory::generate_memory_linker;
use std::{env, fs, path::PathBuf};

fn main() {
    let layout = <Board as BoardMemory>::MEMORY;

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    // memory.x
    let memory_x = generate_memory_linker(&layout);
    fs::write(out_dir.join("memory.x"), memory_x).unwrap();

    println!("cargo:rustc-link-search={}", out_dir.display());

    println!("cargo:rerun-if-changed=build.rs");

    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
    println!("cargo:rustc-link-arg-bins=-Tdefmt.x");
}
