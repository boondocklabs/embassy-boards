use std::path::PathBuf;

use clap::{Parser, Subcommand};
use embassy_boards_config::Boards;

/// Embassy board definition tool
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to board definition files
    #[arg(short, long)]
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
    }
}
