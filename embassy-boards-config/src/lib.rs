//! Board definition parser

use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct Boards {
    boards: Vec<BoardDef>,
}

trait Device {
    fn cargo_flags(&self, out: &mut String);
}

impl Boards {
    /// Load board definitions from a directory of .toml files
    pub fn load(dir: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let boards = Boards {
            boards: load_board_defs(dir)?
                .into_iter()
                .map(|(_path, board)| board)
                .collect(),
        };
        Ok(boards)
    }

    /// Return a reference to a board by the specified name, or None if the board is not found
    pub fn board(&self, name: &str) -> Option<&BoardDef> {
        self.boards
            .iter()
            .find(|board| board.name.to_lowercase() == name.to_lowercase())
    }
}

#[derive(Debug, Deserialize)]
pub struct BoardDef {
    pub name: String,
    pub vendor: String,

    /// Set to true if the target has an MPU
    #[serde(default)]
    pub mpu: bool,

    /// LCD display definition
    pub lcd: Option<LcdDef>,
}

impl BoardDef {
    pub fn cargo_flags(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("cargo:rustc-cfg=board=\"{}\"\n", self.name));
        out.push_str(&format!("cargo:rustc-cfg=vendor=\"{}\"\n", self.vendor));
        if self.mpu == true {
            out.push_str("cargo:rustc-cfg=mpu\n");
        }
        self.lcd.iter().for_each(|lcd| lcd.cargo_flags(&mut out));
        out
    }
}

#[derive(Debug, Deserialize)]
pub struct LcdDef {
    panel: String,
}

impl Device for LcdDef {
    fn cargo_flags(&self, out: &mut String) {
        out.push_str("cargo:rustc-cfg=lcd\n");
        out.push_str(&format!("cargo:rustc-cfg=panel=\"{}\"\n", self.panel));
    }
}

fn find_toml_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            find_toml_files(&path, out)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            out.push(path);
        }
    }
    Ok(())
}

fn load_board_defs(dir: &Path) -> Result<Vec<(PathBuf, BoardDef)>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    find_toml_files(dir, &mut files)?;

    let mut boards = Vec::new();

    for path in files {
        let text = fs::read_to_string(&path)?;
        let board: BoardDef = toml::from_str(&text)?;
        boards.push((path, board));
    }

    Ok(boards)
}
