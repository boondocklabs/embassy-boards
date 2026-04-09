use std::path::PathBuf;

use clap::{Parser, Subcommand};
use embassy_boards_config::prelude::*;

/// Embassy board definition tool
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to board definition files
    #[arg(short, long, default_value = "./boards")]
    path: PathBuf,

    /// Subcommand
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// List board definitions
    List,
    /// Emit cargo flags for the specified board
    CargoFlags { board: String },
    /// Emit generated Rust for a board
    Generate { board: String },
    /// Emit generated memory.x for a board
    GenerateMemory { board: String },
}

pub fn main() {
    let args = Args::parse();
    let boards = Boards::load(&args.path).unwrap();

    match args.command {
        Command::List => {
            println!("{:#?}", boards);
        }
        Command::CargoFlags { board } => {
            if let Some(board) = boards.board(&board) {
                let flags = board.cargo_flags();
                print!("{}", flags)
            } else {
                eprintln!("Board {} not found", board)
            }
        }
        Command::Generate { board } => {
            if let Some(board) = boards.board(&board) {
                let mut out = String::new();
                board.memory.emit_rust(&mut out).unwrap();
                print!("{}", out)
            } else {
                eprintln!("Board {} not found", board)
            }
        }
        Command::GenerateMemory { board } => {
            if let Some(board) = boards.board(&board) {
                let mut out = String::new();
                board.memory.emit_memory_x(&mut out).unwrap();
                print!("{}", out)
            } else {
                eprintln!("Board {} not found", board)
            }
        }
    }
}
