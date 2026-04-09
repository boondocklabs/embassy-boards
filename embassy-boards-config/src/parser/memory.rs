//! Board memory definition parser

use serde::Deserialize;
use std::fmt::Write as _;

mod generate;
mod validate;

#[derive(Debug, Clone, Deserialize)]
pub struct MemoryDef {
    #[serde(default)]
    pub regions: Vec<MemoryRegionDef>,

    #[serde(default)]
    pub aliases: Vec<MemoryAliasDef>,
}

const IMPORTS: &str = r#"
use embassy_boards_core::prelude::*;
use crate::Memory;
"#;

impl MemoryDef {
    pub fn emit_rust(&self, out: &mut String) -> Result<(), String> {
        out.push_str(IMPORTS);

        for region in &self.regions {
            if !region.sections.is_empty() {
                let const_name = region.sections_const_name();
                let _ = writeln!(out, "const {}: &[MemorySectionSpec] = &[", const_name);
                for section in &region.sections {
                    section.emit_rust(out)?;
                }
                out.push_str("];\n\n");
            }
        }

        out.push_str("const REGIONS: &[MemoryRegionSpec] = &[\n");
        for region in &self.regions {
            region.emit_rust(out)?;
        }
        out.push_str("];\n\n");

        out.push_str("const ALIASES: &[MemoryAlias] = &[\n");
        for alias in &self.aliases {
            alias.emit_rust(out);
        }
        out.push_str("];\n\n");

        out.push_str("impl BoardMemory for Memory {\n");
        out.push_str("    const MEMORY: MemoryLayout = MemoryLayout {\n");
        out.push_str("        regions: REGIONS,\n");
        out.push_str("        aliases: ALIASES,\n");
        out.push_str("    };\n");
        out.push_str("}\n");

        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MemoryAliasDef {
    pub name: String,
    pub target: String,
}

impl MemoryAliasDef {
    fn emit_rust(&self, out: &mut String) {
        let _ = writeln!(
            out,
            "    MemoryAlias {{ name: {:?}, target: {:?} }},",
            self.name, self.target
        );
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MemoryRegionDef {
    pub name: String,
    pub origin: IntOrString,
    pub length: IntOrString,
    pub kind: RegionKindDef,

    #[serde(default)]
    pub mpu: Option<MpuAttrsDef>,

    #[serde(default)]
    pub sections: Vec<MemorySectionDef>,
}

impl MemoryRegionDef {
    fn emit_rust(&self, out: &mut String) -> Result<(), String> {
        out.push_str("    MemoryRegionSpec {\n");
        let _ = writeln!(out, "        name: {:?},", self.name);
        let _ = writeln!(out, "        origin: 0x{:08X},", self.origin_u64()?);
        let _ = writeln!(out, "        length: {},", self.length_u64()?);
        let _ = writeln!(out, "        kind: {},", self.kind.rust_expr());

        match &self.mpu {
            Some(mpu) => {
                let _ = writeln!(out, "        mpu: Some({}),", mpu.rust_expr());
            }
            None => {
                out.push_str("        mpu: None,\n");
            }
        }

        if self.sections.is_empty() {
            out.push_str("        sections: None,\n");
        } else {
            let _ = writeln!(
                out,
                "        sections: Some({}),",
                self.sections_const_name()
            );
        }

        out.push_str("    },\n");
        Ok(())
    }

    fn sections_const_name(&self) -> String {
        let mut s = String::from("__SECTIONS_");
        for ch in self.name.chars() {
            if ch.is_ascii_alphanumeric() {
                s.push(ch.to_ascii_uppercase());
            } else {
                s.push('_');
            }
        }
        s
    }

    pub fn origin_u64(&self) -> Result<u64, String> {
        self.origin.parse_u64()
    }

    pub fn length_u64(&self) -> Result<u64, String> {
        self.length.parse_u64()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MemorySectionDef {
    pub name: String,
    pub length: IntOrString,

    #[serde(default)]
    pub origin: Option<IntOrString>,

    #[serde(default)]
    pub align: Option<u64>,
}

impl MemorySectionDef {
    fn emit_rust(&self, out: &mut String) -> Result<(), String> {
        let _ = writeln!(
            out,
            "    MemorySectionSpec::new({:?}, {}),",
            self.name,
            self.length_u64()?
        );
        Ok(())
    }

    pub fn length_u64(&self) -> Result<u64, String> {
        self.length.parse_u64()
    }

    pub fn origin_u64(&self) -> Result<Option<u64>, String> {
        match &self.origin {
            Some(v) => v.parse_u64().map(Some),
            None => Ok(None),
        }
    }

    pub fn align(&self) -> u64 {
        self.align.unwrap_or(1).max(1)
    }

    pub fn linker_section_name(&self) -> String {
        format!(".{}", self.name)
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RegionKindDef {
    Flash,
    Ram,
    Reserved,
    ExternalRam,
}

impl RegionKindDef {
    fn rust_expr(self) -> &'static str {
        match self {
            Self::Flash => "RegionKind::Flash",
            Self::Ram => "RegionKind::Ram",
            Self::Reserved => "RegionKind::Reserved",
            Self::ExternalRam => "RegionKind::ExternalRam",
        }
    }
}

impl MemoryRegionDef {
    fn linker_flags(&self) -> &'static str {
        match self.kind {
            RegionKindDef::Flash => "rx",
            RegionKindDef::Ram => "rwx",
            RegionKindDef::ExternalRam => "rwx",
            RegionKindDef::Reserved => "",
        }
    }

    pub fn resolve_sections(&self) -> Result<Vec<ResolvedSection<'_>>, String> {
        let region_origin = self.origin_u64()?;
        let region_len = self.length_u64()?;
        let region_end = region_origin
            .checked_add(region_len)
            .ok_or_else(|| format!("region {:?} overflows address space", self.name))?;

        let mut out = Vec::new();
        let mut cursor = 0u64;

        for section in &self.sections {
            let len = section.length_u64()?;
            let align = section.align();

            let offset = if let Some(abs_origin) = section.origin_u64()? {
                if abs_origin < region_origin {
                    return Err(format!(
                        "section {:?} origin 0x{:08X} is before region {:?} origin 0x{:08X}",
                        section.name, abs_origin, self.name, region_origin
                    ));
                }

                let rel = abs_origin - region_origin;
                let aligned = align_up(rel, align).ok_or_else(|| {
                    format!(
                        "alignment overflow for section {:?} in region {:?}",
                        section.name, self.name
                    )
                })?;

                if aligned != rel {
                    return Err(format!(
                        "section {:?} origin 0x{:08X} is not aligned to {}",
                        section.name, abs_origin, align
                    ));
                }

                rel
            } else {
                cursor = align_up(cursor, align)
                    .ok_or_else(|| format!("alignment overflow in region {:?}", self.name))?;
                cursor
            };

            let end = offset
                .checked_add(len)
                .ok_or_else(|| format!("section overflow in region {:?}", self.name))?;

            let abs_end = region_origin
                .checked_add(end)
                .ok_or_else(|| format!("section end overflow in region {:?}", self.name))?;

            if abs_end > region_end {
                return Err(format!(
                    "section {:?} exceeds region {:?}: end 0x{:08X} > 0x{:08X}",
                    section.name, self.name, abs_end, region_end
                ));
            }

            out.push(ResolvedSection { section, offset });

            if section.origin.is_none() {
                cursor = end;
            }
        }

        out.sort_by_key(|s| s.offset);

        for pair in out.windows(2) {
            let a = &pair[0];
            let b = &pair[1];
            let a_end = a.offset + a.section.length_u64()?;
            if a_end > b.offset {
                return Err(format!(
                    "sections {:?} and {:?} overlap in region {:?}",
                    a.section.name, b.section.name, self.name
                ));
            }
        }

        Ok(out)
    }
}

pub struct ResolvedSection<'a> {
    pub section: &'a MemorySectionDef,
    pub offset: u64,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct MpuAttrsDef {
    #[serde(default)]
    pub tex: u8,

    #[serde(default = "default_true")]
    pub executable: bool,

    #[serde(default)]
    pub shareable: bool,

    #[serde(default)]
    pub cacheable: bool,

    #[serde(default)]
    pub bufferable: bool,
}

impl MpuAttrsDef {
    fn rust_expr(self) -> String {
        format!(
            "MpuAttrs::new({}, {}, {}, {}, {})",
            self.tex, self.executable, self.shareable, self.cacheable, self.bufferable
        )
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum IntOrString {
    Int(u64),
    String(String),
}

impl IntOrString {
    pub fn parse_u64(&self) -> Result<u64, String> {
        match self {
            Self::Int(v) => Ok(*v),
            Self::String(s) => parse_expr_u64(s),
        }
    }
}

fn parse_expr_u64(s: &str) -> Result<u64, String> {
    // Very small expression parser:
    // supports additions/subtractions of terms:
    // - "256B"
    // - "256KiB"
    // - "2MiB - 256B"
    // - "0x1000_0100"
    // - "2096896"
    //
    // No precedence beyond left-to-right + and -.

    let s = s.trim();
    if s.is_empty() {
        return Err("empty value".to_string());
    }

    let tokens = tokenize_expr(s)?;
    let mut iter = tokens.into_iter();

    let first = iter
        .next()
        .ok_or_else(|| "expected first term".to_string())?;
    let mut acc = parse_term_u64(&first)?;

    while let Some(op) = iter.next() {
        let rhs = iter
            .next()
            .ok_or_else(|| format!("operator {op:?} missing rhs"))?;
        let rhs = parse_term_u64(&rhs)?;

        match op.as_str() {
            "+" => {
                acc = acc
                    .checked_add(rhs)
                    .ok_or_else(|| format!("overflow in expression: {s}"))?;
            }
            "-" => {
                acc = acc
                    .checked_sub(rhs)
                    .ok_or_else(|| format!("underflow in expression: {s}"))?;
            }
            _ => return Err(format!("unexpected operator: {op}")),
        }
    }

    Ok(acc)
}

fn tokenize_expr(s: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut cur = String::new();

    for ch in s.chars() {
        match ch {
            '+' | '-' => {
                if !cur.trim().is_empty() {
                    out.push(cur.trim().to_string());
                }
                out.push(ch.to_string());
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }

    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }

    if out.is_empty() {
        return Err("empty expression".to_string());
    }

    Ok(out)
}

fn parse_term_u64(term: &str) -> Result<u64, String> {
    let term = term.trim();
    if term.is_empty() {
        return Err("empty term".to_string());
    }

    // Hex literal: allow only 0x... with optional underscores, no size suffix.
    if term.starts_with("0x") || term.starts_with("0X") {
        let num = parse_number_u64(term)?;
        return Ok(num);
    }

    // Decimal number with optional suffix like B, KiB, MiB, etc.
    let mut split_idx = term.len();
    for (i, ch) in term.char_indices() {
        if !(ch.is_ascii_digit() || ch == '_') {
            split_idx = i;
            break;
        }
    }

    let (num_part, suffix_part) = term.split_at(split_idx);
    let num_part = num_part.trim();
    let suffix_part = suffix_part.trim();

    if num_part.is_empty() {
        return Err(format!("missing number in term: {term}"));
    }

    let num = parse_number_u64(num_part)?;
    let mult = parse_size_suffix(suffix_part)?;

    num.checked_mul(mult)
        .ok_or_else(|| format!("overflow in term: {term}"))
}

fn parse_number_u64(s: &str) -> Result<u64, String> {
    let cleaned = s.replace('_', "");

    if cleaned.starts_with("0x") || cleaned.starts_with("0X") {
        u64::from_str_radix(&cleaned[2..], 16)
            .map_err(|e| format!("invalid hex integer {s:?}: {e}"))
    } else {
        cleaned
            .parse::<u64>()
            .map_err(|e| format!("invalid integer {s:?}: {e}"))
    }
}

fn parse_size_suffix(s: &str) -> Result<u64, String> {
    let s = s.trim();

    if s.is_empty() {
        return Ok(1);
    }

    match s {
        "B" | "b" => Ok(1),
        "KiB" | "KIB" | "kib" => Ok(1024),
        "MiB" | "MIB" | "mib" => Ok(1024 * 1024),
        "GiB" | "GIB" | "gib" => Ok(1024 * 1024 * 1024),
        "KB" | "kb" => Ok(1000),
        "MB" | "mb" => Ok(1000 * 1000),
        "GB" | "gb" => Ok(1000 * 1000 * 1000),
        _ => Err(format!("unknown size suffix: {s:?}")),
    }
}

fn align_up(value: u64, align: u64) -> Option<u64> {
    if align <= 1 {
        return Some(value);
    }
    let mask = align - 1;
    value.checked_add(mask).map(|v| v & !mask)
}
