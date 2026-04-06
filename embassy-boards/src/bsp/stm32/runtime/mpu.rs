//! Cortex M7 MPU

use crate::memory::BoardMemory;
use cortex_m::{Peripherals, asm, peripheral::MPU};

use crate::BoardConfig;

// https://developer.arm.com/documentation/dui0646/c/Cortex-M7-Peripherals/Optional-Memory-Protection-Unit/MPU-Region-Attribute-and-Size-Register
// https://developer.arm.com/documentation/dui0646/c/Cortex-M7-Peripherals/Optional-Memory-Protection-Unit/MPU-access-permission-attributes

const MPU_XN: u32 = 1 << 28;
const MPU_ENABLE: u32 = 1;
const MPU_AP_FULL_ACCESS: u32 = (0b011) << 24;

const MPU_DEFAULT_MMAP_FOR_PRIVILEGED: u32 = 0x04;

const fn mpu_size_field(size: usize) -> u32 {
    size.ilog2() - 1
}

pub fn mpu_attrs(tex: u8, c: bool, b: bool, s: bool) -> u32 {
    ((tex as u32 & 0b111) << 19) | (s as u32) << 18 | (c as u32) << 17 | (b as u32) << 16
}

pub unsafe fn mpu_region(mpu: &mut MPU, region: u32, base: usize, size: usize, attrs: u32) {
    debug_assert!(size.is_power_of_two());
    debug_assert_eq!(base & (size - 1), 0);

    let rasr: u32 = attrs | MPU_AP_FULL_ACCESS | MPU_XN | (mpu_size_field(size) << 1) | MPU_ENABLE;

    #[cfg(feature = "defmt")]
    defmt::debug!(
        "MPU region: {} base: {:x} size: {} rasr: {:b} attrs: {:b}",
        region,
        base,
        size,
        rasr,
        attrs
    );

    unsafe {
        mpu.rnr.write(region);
        mpu.rbar.write(base as u32 | (1 << 4) | (region & 0b1111));
        mpu.rasr.write(rasr);
    }
    cortex_m::asm::dsb();
    cortex_m::asm::isb();
}

pub fn init_mpu<Board: BoardConfig>(cp: &mut Peripherals) {
    cp.SCB.disable_icache();
    cp.SCB.disable_dcache(&mut cp.CPUID);
    asm::dmb();
    asm::isb();

    unsafe { cp.MPU.ctrl.write(0) };

    let mut index = 0;
    for region in <Board as BoardConfig>::Layout::MEMORY.regions {
        if let Some(attrs) = region.mpu {
            let attrs = mpu_attrs(
                attrs.tex,
                attrs.cacheable,
                attrs.bufferable,
                attrs.shareable,
            );

            unsafe {
                mpu_region(&mut cp.MPU, index, region.origin, region.length, attrs);
            }

            index += 1;
        }

        if let Some(sections) = region.sections {
            for section in sections {
                // Get const resolved section for origin
                let s = <Board as BoardConfig>::Layout::MEMORY
                    .section(region.name, section.name)
                    .unwrap();

                if let Some(attrs) = s.mpu {
                    let attrs = mpu_attrs(
                        attrs.tex,
                        attrs.cacheable,
                        attrs.bufferable,
                        attrs.shareable,
                    );

                    unsafe {
                        mpu_region(&mut cp.MPU, index, s.origin, s.length, attrs);
                    }

                    index += 1;
                }
            }
        }
    }

    unsafe {
        cp.MPU
            .ctrl
            .write(MPU_DEFAULT_MMAP_FOR_PRIVILEGED | MPU_ENABLE)
    };

    cp.SCB.enable_icache();
    cp.SCB.enable_dcache(&mut cp.CPUID);

    cortex_m::asm::dsb();
    cortex_m::asm::isb();
}
