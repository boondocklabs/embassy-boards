use super::MemoryDef;
use super::RegionKindDef;

impl MemoryDef {
    /// Generate a `memory.x` linker script from a [`MemoryDef`]
    pub fn emit_memory_x(&self, out: &mut String) -> Result<(), String> {
        use std::fmt::Write as _;

        self.validate()?;

        out.push_str("MEMORY\n{\n");

        for region in &self.regions {
            let flags = region.linker_flags();
            if flags.is_empty() {
                let _ = writeln!(
                    out,
                    "  {} : ORIGIN = 0x{:08X}, LENGTH = {}",
                    region.name,
                    region.origin_u64()?,
                    region.length_u64()?
                );
            } else {
                let _ = writeln!(
                    out,
                    "  {} ({}) : ORIGIN = 0x{:08X}, LENGTH = {}",
                    region.name,
                    flags,
                    region.origin_u64()?,
                    region.length_u64()?
                );
            }
        }

        out.push_str("}\n\n");
        out.push_str("SECTIONS\n{\n");

        for region in &self.regions {
            let base = region.origin_u64()?;

            for resolved in region.resolve_sections()? {
                let section = resolved.section;
                let sec_name = section.linker_section_name();
                let align = section.align();
                let abs = base.checked_add(resolved.offset).ok_or_else(|| {
                    format!("section address overflow in region {:?}", region.name)
                })?;

                let noload = match region.kind {
                    RegionKindDef::Flash => "",
                    _ => " (NOLOAD)",
                };

                let _ = writeln!(out, "  {} 0x{:08X}{} :", sec_name, abs, noload);
                out.push_str("  {\n");
                let _ = writeln!(out, "    . = ALIGN({});", align);
                let _ = writeln!(out, "    KEEP(*({}));", sec_name);
                let _ = writeln!(out, "    KEEP(*({}.*));", sec_name);
                let _ = writeln!(out, "    . = ALIGN({});", align);
                let _ = writeln!(out, "  }} > {}\n", region.name);
            }
        }

        out.push_str("}\n");

        if !self.aliases.is_empty() {
            out.push_str("\n/* Optional aliases */\n");
            for alias in &self.aliases {
                let target = self
                    .regions
                    .iter()
                    .find(|r| r.name == alias.target)
                    .ok_or_else(|| {
                        format!(
                            "alias {:?} points to unknown region {:?}",
                            alias.name, alias.target
                        )
                    })?;

                let _ = writeln!(out, "REGION_ALIAS({:?}, {});", alias.name, target.name);
            }
        }

        Ok(())
    }
}
