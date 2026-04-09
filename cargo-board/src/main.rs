use std::path::PathBuf;

use clap::{CommandFactory, Parser, Subcommand, error::ErrorKind};
use color_eyre::owo_colors::OwoColorize;
use embassy_boards_config::prelude::*;

/// Board definition tool
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to board definition files
    #[arg(short, long, env = "EMBASSY_BOARD_PATH")]
    path: PathBuf,

    /// Board ID
    #[arg(short, long, env = "EMBASSY_BOARD_ID")]
    board: Option<String>,

    /// Subcommand
    #[command(subcommand)]
    action: Action,
}

#[derive(Debug, Subcommand, PartialEq, Eq, PartialOrd, Ord)]
enum Action {
    /// List board definitions
    List,
    /// Emit cargo flags for the specified board
    CargoFlags {},
    /// Emit generated Rust for a board
    Generate {},
    /// Emit generated memory.x for a board
    GenerateMemory {},
    Run {},
}

pub fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    // If invoked from cargo, "board" will be prepended as the first argument.
    // Remove the first argument before invoking the clap parser
    let mut args: Vec<_> = std::env::args_os().collect();

    if args.get(1).and_then(|s| s.to_str()) == Some("board") {
        args.remove(1);
    }

    let args = Args::parse_from(args);
    let boards = Boards::load(&args.path)?;

    if args.action == Action::List {
        // Compute column widths
        let id_width = boards.iter().map(|b| b.id.len()).max().unwrap_or(2).max(2);

        let name_width = boards
            .iter()
            .map(|b| b.name.len())
            .max()
            .unwrap_or(4)
            .max(4);

        // Header
        println!(
            "{:<id_width$}  {:<name_width$}",
            "ID",
            "NAME",
            id_width = id_width,
            name_width = name_width
        );

        println!(
            "{:-<id_width$}  {:-<name_width$}",
            "",
            "",
            id_width = id_width,
            name_width = name_width
        );

        // Rows
        for board in &boards {
            println!(
                "{:<id_width$}  {:<name_width$}",
                board.id.bold().cyan(),
                board.name.bold(),
                id_width = id_width,
                name_width = name_width
            );
        }

        return Ok(());
    }

    let Some(board_id) = args.board else {
        Args::command()
            .error(ErrorKind::MissingRequiredArgument, "--board is required")
            .exit();
    };
    let Some(board) = boards.board(&board_id) else {
        Args::command()
            .error(
                ErrorKind::InvalidValue,
                format!("Board ID {} not found", board_id),
            )
            .exit();
    };

    match args.action {
        Action::List => {}
        Action::CargoFlags {} => {
            let flags = board.cargo_flags();
            print!("{}", flags);
        }
        Action::Generate {} => {
            let mut out = String::new();
            board.memory.emit_rust(&mut out).unwrap();
            print!("{}", out);
        }
        Action::GenerateMemory {} => {
            let mut out = String::new();
            board.memory.emit_memory_x(&mut out).unwrap();
        }
        Action::Run {} => {
            println!();
            println!("=== Running Board ===");
            println!("Board    : {}", board.name);
            println!("Platform : {}", board.platform);
            println!("Target   : {}", board.target);
            println!("Chip     : {}", board.chip);

            if !board.features.is_empty() {
                println!("Features: {}", board.features.join(", "));
            }

            println!("=====================");
            println!();

            let mut cmd = std::process::Command::new("cargo");

            cmd.env("EMBASSY_BOARD", &board.id)
                .arg("run")
                .arg("--target")
                .arg(&board.target);

            // Features
            if !board.features.is_empty() {
                cmd.arg("--features").arg(board.features.join(","));
            }

            cmd.arg("--release");

            // Pass through extra args after `--`
            /*
            if let Some(extra) = &args.extra_args {
                cmd.arg("--");
                cmd.args(extra);
            }
            */

            println!("> {:?}", cmd);

            let status = cmd.status()?;

            if !status.success() {
                color_eyre::eyre::bail!("cargo run failed");
            }
        }
    }

    Ok(())
}
