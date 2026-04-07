use std::path::PathBuf;

use embassy_boards_config::Boards;

fn main() {
    let boards = Boards::load(&PathBuf::from("../embassy-boards-config/boards")).unwrap();
    let board = boards.board("STM32F429i-DISCO").unwrap();

    /*
    let board = boards
        .board("STM32H747I-DISCO CM7")
        .expect("Board not found");
        */

    let flags = board.cargo_flags();

    /*
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    // memory.x
    let memory_x = generate_memory_linker(&layout);
    fs::write(out_dir.join("memory.x"), memory_x).unwrap();

    println!("cargo:rustc-link-search={}", out_dir.display());

    println!("cargo:rerun-if-changed=build.rs");

    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
    println!("cargo:rustc-link-arg-bins=-Tdefmt.x");
    */
    print!("{}", flags);
}
