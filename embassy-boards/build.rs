use std::{env, fs, path::PathBuf};

use embassy_boards_config::prelude::*;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let boards = Boards::load(&PathBuf::from("../embassy-boards-config/boards")).unwrap();
    let board = boards
        .board(
            &env::var_os("EMBASSY_BOARD")
                .expect("EMBASSY_BOARD environment not set")
                .into_string()
                .unwrap(),
        )
        .expect("Board not found");

    if let Err(e) = board.memory.validate() {
        panic!("Invalid memory layout: {}", e);
    }

    let mut memory = String::new();
    if let Err(e) = board.memory.emit_rust(&mut memory) {
        panic!("Failed to generate memory.rs: {}", e);
    }

    let out_file = out_dir.join("memory.rs");
    fs::write(&out_file, memory).unwrap();

    let flags = board.cargo_flags();
    print!("{}", flags);

    let mut memory_x = String::new();
    board.memory.emit_memory_x(&mut memory_x).unwrap();
    fs::write(out_dir.join("memory.x"), memory_x).unwrap();
    println!("cargo:rustc-link-search={}", out_dir.display());

    println!("cargo:rerun-if-changed=build.rs");
}
