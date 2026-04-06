use embassy_stm32::{exti::ExtiInput, mode::Async};
use embedded_hal::i2c::Operation;
use embedded_hal_async::i2c::I2c;

// Use SRAM4 for I2C4 BDMA
#[unsafe(link_section = ".bdma")]
static mut TX: [u8; 1] = [0; _];

#[unsafe(link_section = ".bdma")]
static mut RX: [u8; 32] = [0; _];

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TouchPoint {
    pub id: u8,
    pub event: TouchEvent,
    pub x: u16,
    pub y: u16,
    pub weight: u8,
    pub misc: u8,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum TouchEvent {
    Down = 0,
    Up = 1,
    Contact = 2,
    Unknown(u8),
}

impl TouchEvent {
    fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Down,
            1 => Self::Up,
            2 => Self::Contact,
            _ => Self::Unknown(value),
        }
    }
}

#[derive(Clone, Copy)]
#[repr(u8)]
#[allow(unused)]
enum Register {
    DeviceMode = 0x00,
    GestureId = 0x01,
    TouchCount = 0x02,
    TouchData = 0x03,
    ChipId = 0xA3,
    FirmwareId = 0xA6,
}

pub struct Ft5316<BUS: I2c, const ADDR: u8 = 0x38, const TOUCH_POINTS: usize = 2> {
    bus: BUS,
    int: ExtiInput<'static, Async>,
}

impl<BUS: I2c, const ADDR: u8, const TOUCH_POINTS: usize> Ft5316<BUS, ADDR, TOUCH_POINTS> {
    pub fn new(bus: BUS, int: ExtiInput<'static, Async>) -> Self {
        Self { bus, int }
    }

    #[allow(static_mut_refs)]
    async fn read_register(&mut self, reg: Register, dest: &mut [u8]) -> Result<(), BUS::Error> {
        unsafe {
            TX[0] = reg as u8;
            self.bus
                .transaction(
                    ADDR,
                    &mut [
                        Operation::Write(&TX),
                        Operation::Read(&mut RX[..dest.len()]),
                    ],
                )
                .await?;

            dest.copy_from_slice(&RX[..dest.len()]);
        }
        Ok(())
    }

    #[cfg(feature = "defmt")]
    #[allow(static_mut_refs)]
    pub async fn dump_regs(&mut self) {
        let mut buf = [0u8; 0x10];
        unsafe {
            TX[0] = 0x00;
            self.bus
                .transaction(
                    ADDR,
                    &mut [Operation::Write(&TX), Operation::Read(&mut RX[..buf.len()])],
                )
                .await
                .unwrap();

            buf.copy_from_slice(&RX[..0x10]);
        }

        for (i, b) in buf.iter().enumerate() {
            defmt::info!("reg {:02X} = {:02X}", i, b);
        }
    }

    pub async fn read_touches(
        &mut self,
        out: &mut [Option<TouchPoint>; TOUCH_POINTS],
    ) -> Result<usize, BUS::Error> {
        // Wait for the interrupt
        self.int.wait_for_falling_edge().await;

        for slot in out.iter_mut() {
            *slot = None;
        }

        // Read TD_STATUS + touch records in one burst.
        let mut buf = [0u8; 1 + 5 * 6];
        self.read_register(Register::TouchCount, &mut buf).await?;

        let count = (buf[0] & 0x0F).min(TOUCH_POINTS as u8) as usize;

        for i in 0..count {
            let base = 1 + i * 6;
            let p = &buf[base..base + 6];

            let event = p[0] >> 6;
            let x = (((p[0] & 0x0F) as u16) << 8) | p[1] as u16;
            let y = (((p[2] & 0x0F) as u16) << 8) | p[3] as u16;
            let id = (p[2] >> 4) & 0x0F;

            out[i] = Some(TouchPoint {
                id,
                event: TouchEvent::from_u8(event),
                x,
                y,
                weight: p[4],
                misc: p[5],
            });
        }

        Ok(count)
    }

    pub async fn init(&mut self) {
        let mut buf = [0u8];
        self.read_register(Register::ChipId, &mut buf)
            .await
            .unwrap();

        #[cfg(feature = "defmt")]
        defmt::info!("ChipId={:X}", buf);

        self.read_register(Register::FirmwareId, &mut buf)
            .await
            .unwrap();

        #[cfg(feature = "defmt")]
        defmt::info!("FirmwareId={:X}", buf);
    }
}
