use crate::memory::{
    BoardMemory, MemoryAlias, MemoryLayout, MpuAttrs,
    region::{MemoryRegionSpec, RegionKind},
    section::MemorySectionSpec,
};

const REGIONS: &[MemoryRegionSpec] = &[
    MemoryRegionSpec {
        name: "FLASH_CM7",
        origin: 0x0800_0000,
        length: 1 * 1024 * 1024,
        kind: RegionKind::Flash,
        mpu: None,
        sections: None,
    },
    MemoryRegionSpec {
        name: "FLASH_CM4",
        origin: 0x0810_0000,
        length: 1 * 1024 * 1024,
        kind: RegionKind::Flash,
        mpu: None,
        sections: None,
    },
    MemoryRegionSpec {
        name: "AXIRAM",
        origin: 0x2400_0000,
        length: 512 * 1024,
        kind: RegionKind::Ram,
        mpu: None,
        sections: None,
    },
    MemoryRegionSpec {
        name: "SRAM1",
        origin: 0x1000_0000,
        length: 128 * 1024,
        kind: RegionKind::Ram,
        mpu: None,
        sections: None,
    },
    MemoryRegionSpec {
        name: "SRAM4",
        origin: 0x3800_0000,
        length: 64 * 1024,
        kind: RegionKind::Ram,
        mpu: None,
        sections: Some(&[
            MemorySectionSpec::new("shared_data", 1024),
            MemorySectionSpec::new("rtt", 8192),
            MemorySectionSpec::new("bdma", 2048)
                .with_mpu(MpuAttrs::new(0b001, false, false, false, false)),
            // Section for message queue from CM7 to CM4
            MemorySectionSpec::new("cm7_to_cm4", 24576),
            // Section for message queue from CM4 to CM7
            MemorySectionSpec::new("cm4_to_cm7", 24567),
        ]),
    },
    MemoryRegionSpec {
        name: "SDRAM",
        origin: 0xD000_0000,
        length: 32 * 1024 * 1024,
        kind: RegionKind::ExternalRam,
        mpu: None,
        sections: Some(&[
            MemorySectionSpec::new("fb0", 2 * 1024 * 1024)
                .with_mpu(MpuAttrs::new(0b001, false, false, false, false)),
            MemorySectionSpec::new("fb1", 2 * 1024 * 1024)
                .with_mpu(MpuAttrs::new(0b001, false, false, false, false)),
            MemorySectionSpec::new("tex", 12 * 1024 * 1024)
                .with_mpu(MpuAttrs::new(0b001, false, false, false, false)),
            MemorySectionSpec::new("heap", 16 * 1024 * 1024)
                .with_align(16)
                .with_mpu(MpuAttrs::new(0b000, false, false, true, true)),
        ]),
    },
    MemoryRegionSpec {
        name: "QSPI",
        origin: 0x9000_0000,
        length: 128 * 1024 * 1024,
        kind: RegionKind::ExternalFlash,
        mpu: None,
        sections: None,
    },
];

#[cfg(all(feature = "board-stm32h747i-disco", feature = "cpu0"))]
impl BoardMemory for super::Stm32h747iCm7Memory {
    const MEMORY: crate::memory::MemoryLayout = MemoryLayout {
        regions: REGIONS,
        aliases: &[
            MemoryAlias {
                name: "FLASH",
                target: "FLASH_CM7",
            },
            MemoryAlias {
                name: "RAM",
                target: "AXIRAM",
            },
        ],
    };
}

#[cfg(all(feature = "board-stm32h747i-disco", feature = "cpu1"))]
impl BoardMemory for super::Stm32h747iCm4Memory {
    const MEMORY: crate::memory::MemoryLayout = MemoryLayout {
        regions: REGIONS,
        aliases: &[
            MemoryAlias {
                name: "FLASH",
                target: "FLASH_CM4",
            },
            MemoryAlias {
                name: "RAM",
                target: "SRAM1",
            },
        ],
    };
}
