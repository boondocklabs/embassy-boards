use crate::memory::{
    BoardMemory, MemoryAlias, MemoryLayout,
    region::{MemoryRegionSpec, RegionKind},
    section::MemorySectionSpec,
};

const REGIONS: &[MemoryRegionSpec] = &[
    // 64KB Core Coupled Memory
    MemoryRegionSpec {
        name: "CCM",
        origin: 0x1000_0000,
        length: 64 * 1024,
        kind: RegionKind::Ram,
        mpu: None,
        sections: None,
    },
    MemoryRegionSpec {
        name: "FLASH",
        origin: 0x0800_0000,
        length: 1 * 1024 * 1024,
        kind: RegionKind::Flash,
        mpu: None,
        sections: None,
    },
    MemoryRegionSpec {
        name: "SRAM",
        origin: 0x2000_0000,
        length: 192 * 1024,
        kind: RegionKind::Ram,
        mpu: None,
        sections: None,
        //sections: Some(&[MemorySectionSpec::new("fb", 192 * 1024)]),
    },
    /*
    MemoryRegionSpec {
        name: "SRAM2",
        origin: 0x2001_C000,
        length: 16 * 1024,
        kind: RegionKind::Ram,
        mpu: None,
        sections: None,
    },
    MemoryRegionSpec {
        name: "SRAM3",
        origin: 0x2002_0000,
        length: 64 * 1024,
        kind: RegionKind::Ram,
        mpu: None,
        sections: None,
    },
    */
    MemoryRegionSpec {
        name: "SDRAM",
        origin: 0xD000_0000,
        length: 8 * 1024 * 1024,
        kind: RegionKind::ExternalRam,
        mpu: None,
        sections: Some(&[
            MemorySectionSpec::new("fb0", 512 * 1024),
            MemorySectionSpec::new("fb1", 512 * 1024),
            MemorySectionSpec::new("tex", 1 * 1024 * 1024),
            MemorySectionSpec::new("heap", 6 * 1024 * 1024),
        ]),
    },
];

impl BoardMemory for super::Memory {
    const MEMORY: crate::memory::MemoryLayout = MemoryLayout {
        regions: REGIONS,
        aliases: &[MemoryAlias {
            name: "RAM",
            target: "CCM",
        }],
    };
}
