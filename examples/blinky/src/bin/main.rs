#![no_std]
#![no_main]

use boards::BoardConfig;
use boards::Board;
use boards::cortex_m_rt;
use embassy_boards as boards;
use embassy_boards::embassy_time::Timer;
use embassy_executor::Spawner;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let mut devices = Board::init().await;

    defmt::info!("{} initialized", Board::NAME);

    let mut count = 0u32;
    loop {
        devices.led.mask(count);
        count = count.wrapping_add(1);
        Timer::after_millis(50).await;
    }
}
