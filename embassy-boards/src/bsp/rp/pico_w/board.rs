use crate::{
    Board, BoardConfig,
    drivers::{BoardDrivers, led::Led},
};

use cyw43::{A4, Aligned, aligned_bytes};
use cyw43_pio::{PioSpi, RM2_CLOCK_DIVIDER};
use embassy_executor::{Executor, Spawner};
use embassy_rp::{
    adc, bind_interrupts,
    clocks::RoscRng,
    dma,
    gpio::{Level, Output},
    multicore, peripherals,
    pio::{self, Pio},
};
use embassy_time::Timer;
use static_cell::StaticCell;

const WIFI_NETWORK: Option<&str> = option_env!("WIFI_NETWORK");
const WIFI_PASSWORD: Option<&str> = option_env!("WIFI_PASSWORD");

static mut CORE1_STACK: multicore::Stack<32768> = multicore::Stack::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();

#[unsafe(link_section = ".firmware")]
static FW: &Aligned<A4, [u8]> =
    aligned_bytes!("../../../../../../embassy/cyw43-firmware/43439A0.bin");
#[unsafe(link_section = ".firmware")]
static CLM: &Aligned<A4, [u8]> =
    aligned_bytes!("../../../../../../embassy/cyw43-firmware/43439A0_clm.bin");
#[unsafe(link_section = ".firmware")]
static NVRAM: &Aligned<A4, [u8]> =
    aligned_bytes!("../../../../../../embassy/cyw43-firmware/nvram_rp2040.bin");

static CYW43_STATE: StaticCell<cyw43::State> = StaticCell::new();

bind_interrupts!(struct Irqs {
    ADC_IRQ_FIFO => adc::InterruptHandler;
    PIO0_IRQ_0 => pio::InterruptHandler<peripherals::PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<peripherals::DMA_CH0>, dma::InterruptHandler<peripherals::DMA_CH1>, dma::InterruptHandler<peripherals::DMA_CH2>;
});

pub struct Devices {
    pub led: <Board as BoardDrivers>::Led,
}

impl BoardDrivers for Board {
    type Led = Led<embassy_rp::gpio::Output<'static>, 3>;
}

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<
        'static,
        cyw43::SpiBus<Output<'static>, PioSpi<'static, peripherals::PIO0, 0>>,
    >,
) -> ! {
    //defmt::info!("CYW43 Task Started");
    Timer::after_millis(100).await;
    runner.run().await;
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    defmt::info!("Network task started");
    Timer::after_millis(100).await;
    runner.run().await
}

#[embassy_executor::task]
async fn wifi_task(
    spawner: Spawner,
    wifi_spi: PioSpi<'static, peripherals::PIO0, 0>,
    wifi_pwr: Output<'static>,
) {
    defmt::info!("Wifi task started on CPU1");

    let state = CYW43_STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, wifi_pwr, wifi_spi, FW, NVRAM).await;

    spawner.spawn(cyw43_task(runner).unwrap());

    control.init(CLM).await;
    control
        .set_power_management(cyw43::PowerManagementMode::Performance)
        .await;

    let mut rng = RoscRng;
    let seed = rng.next_u64();

    // Init network stack
    let config = embassy_net::Config::dhcpv4(Default::default());
    static RESOURCES: StaticCell<embassy_net::StackResources<3>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(embassy_net::StackResources::new()),
        seed,
    );

    spawner.spawn(net_task(runner).unwrap());

    loop {
        if let (Some(network), Some(password)) = (WIFI_NETWORK, WIFI_PASSWORD) {
            defmt::info!("Connecting to {}", network);
            while let Err(_err) = control
                .join(network, cyw43::JoinOptions::new(password.as_bytes()))
                .await
            {
                defmt::info!("WiFi join failed");
                Timer::after_millis(200).await;
            }
        }

        stack.wait_link_up().await;
        defmt::info!("Link up");
        stack.wait_config_up().await;
        defmt::info!("Config up");

        if let Some(config) = stack.config_v4() {
            defmt::info!("IP Address: {}", config.address);
        }

        while stack.is_link_up() && stack.is_config_up() {
            match stack
                .dns_query("google.com", embassy_net::dns::DnsQueryType::A)
                .await
            {
                Ok(response) => {
                    defmt::info!("DNS response {}", response);
                }
                Err(err) => defmt::info!("DNS error {}", err),
            }
            Timer::after_millis(10000).await;
        }

        defmt::info!("Network disconnected");

        control.leave().await;
    }
}

impl BoardConfig for Board {
    const NAME: &str = "RPi Pico W";
    const VENDOR: &str = "Raspberry Pi";

    type Layout = crate::Memory;
    type Devices = Devices;

    async fn init() -> Self::Devices {
        let config = embassy_rp::config::Config::default();
        let p = embassy_rp::init(config);

        // Spawn CPU core 1 and start Wifi on it
        multicore::spawn_core1(
            p.CORE1,
            unsafe { &mut *core::ptr::addr_of_mut!(CORE1_STACK) },
            move || {
                let executor1 = EXECUTOR1.init(Executor::new());
                executor1.run(|spawner| {
                    let mut pio = Pio::new(p.PIO0, Irqs);

                    let wifi_pwr = Output::new(p.PIN_23, Level::Low);
                    let wifi_cs = Output::new(p.PIN_25, Level::High);
                    let wifi_spi = PioSpi::new(
                        &mut pio.common,
                        pio.sm0,
                        RM2_CLOCK_DIVIDER,
                        pio.irq0,
                        wifi_cs,
                        p.PIN_24,
                        p.PIN_29,
                        dma::Channel::new(p.DMA_CH0, Irqs),
                    );
                    spawner.spawn(wifi_task(spawner, wifi_spi, wifi_pwr).unwrap());
                });
            },
        );

        let led1 = Output::new(p.PIN_6, embassy_rp::gpio::Level::Low);
        let led2 = Output::new(p.PIN_7, embassy_rp::gpio::Level::Low);
        let led3 = Output::new(p.PIN_8, embassy_rp::gpio::Level::Low);
        let led = Led {
            pins: [led1, led2, led3],
        };

        Devices { led }
    }

    #[cfg(heap)]
    fn heap_free() -> usize {
        todo!()
    }

    #[cfg(heap)]
    fn heap_used() -> usize {
        todo!()
    }

    #[cfg(heap)]
    fn heap_size() -> usize {
        todo!()
    }
}
