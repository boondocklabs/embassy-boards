use core::slice;

use cortex_m::{Peripherals, asm};
use embassy_stm32::{
    Config, bind_interrupts, dma,
    dma2d::{self, PixelFormat},
    dsihost::{self, panel::DsiPanel},
    exti,
    fmc::Fmc,
    hsem, i2c, interrupt, ltdc, peripherals,
    qspi::{self, Qspi},
    rcc::{DsiHostPllConfig, DsiPllInput, DsiPllOutput, Hse, Pll},
    sdmmc::{self, Sdmmc},
    time::Hertz,
};
use embassy_time::Delay;
use stm32_fmc::{Sdram, devices::is42s32800g_6::Is42s32800g};

use crate::{
    Board, align_up,
    bsp::stm32::{
        SHARED_DATA,
        mpu::{mpu_attrs, mpu_region},
    },
};

pub struct Devices {}

bind_interrupts!(
    struct Irqs {
        HSEM2 => hsem::HardwareSemaphoreInterruptHandler<peripherals::HSEM>;
        DMA1_STREAM3 => dma::InterruptHandler<peripherals::DMA1_CH3>;
        DMA1_STREAM4 => dma::InterruptHandler<peripherals::DMA1_CH4>;
        EXTI9_5 => exti::InterruptHandler<interrupt::typelevel::EXTI9_5>;
        EXTI4 => exti::InterruptHandler<interrupt::typelevel::EXTI4>;
        EXTI3 => exti::InterruptHandler<interrupt::typelevel::EXTI3>;
    }
);

pub struct Board {}

impl BoardConfig for Board {
    const NAME: &str = "STM32H747i-DISCO CPU1 (Cortex-M4)";
    const VENDOR: &str = "ST";

    async fn init() -> Devices {
        let p = embassy_stm32::init_secondary(config, &SHARED_DATA);

        Devices { sdmmc, qspi }
    }
}
