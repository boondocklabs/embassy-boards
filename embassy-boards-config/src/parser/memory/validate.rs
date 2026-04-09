use super::MemoryDef;

impl MemoryDef {
    pub fn validate(&self) -> Result<(), String> {
        for region in &self.regions {
            let origin = region
                .origin_u64()
                .map_err(|e| format!("region {:?} origin: {}", region.name, e))?;
            let length = region
                .length_u64()
                .map_err(|e| format!("region {:?} length: {}", region.name, e))?;

            let end = origin
                .checked_add(length)
                .ok_or_else(|| format!("region {:?} overflows address space", region.name))?;

            let mut used = 0u64;
            for section in &region.sections {
                let len = section.length_u64().map_err(|e| {
                    format!(
                        "region {:?} section {:?} length: {}",
                        region.name, section.name, e
                    )
                })?;
                used = used
                    .checked_add(len)
                    .ok_or_else(|| format!("section total overflow in region {:?}", region.name))?;
            }

            if used > length {
                return Err(format!(
                    "sections in region {:?} exceed region length: used {} > {}",
                    region.name, used, length
                ));
            }

            let _ = end;
        }

        for (i, a) in self.regions.iter().enumerate() {
            let a_origin = a.origin_u64()?;
            let a_len = a.length_u64()?;
            let a_end = a_origin
                .checked_add(a_len)
                .ok_or_else(|| format!("region {:?} overflows address space", a.name))?;

            for b in self.regions.iter().skip(i + 1) {
                let b_origin = b.origin_u64()?;
                let b_len = b.length_u64()?;
                let b_end = b_origin
                    .checked_add(b_len)
                    .ok_or_else(|| format!("region {:?} overflows address space", b.name))?;

                let overlap = a_origin < b_end && b_origin < a_end;
                if overlap {
                    return Err(format!(
                        "memory regions {:?} and {:?} overlap",
                        a.name, b.name
                    ));
                }
            }
        }

        for alias in &self.aliases {
            let exists = self.regions.iter().any(|r| r.name == alias.target);
            if !exists {
                return Err(format!(
                    "alias {:?} points to unknown region {:?}",
                    alias.name, alias.target
                ));
            }
        }

        Ok(())
    }
}
