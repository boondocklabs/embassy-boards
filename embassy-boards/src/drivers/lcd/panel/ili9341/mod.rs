use embassy_time::Timer;
use embedded_hal::{digital::OutputPin, spi::SpiBus};

pub struct Ili9341;

impl super::SpiPanel for Ili9341 {
    async fn init<Bus: SpiBus, CS: OutputPin, DC: OutputPin>(
        bus: &mut Bus,
        cs: &mut CS,
        dc: &mut DC,
    ) {
        ili9341_init(bus, cs, dc).await;
    }
}

// Initialize the IL9341. Commands are derived from https://github.com/STMicroelectronics/stm32-ili9341
async fn ili9341_init(spi: &mut impl SpiBus, cs: &mut impl OutputPin, dc: &mut impl OutputPin) {
    lcd_cmd_data(spi, cs, dc, 0xCA, &[0xC3, 0x08, 0x50]);

    lcd_cmd_data(spi, cs, dc, 0xCF, &[0x00, 0xC1, 0x30]); // LCD_POWERB
    lcd_cmd_data(spi, cs, dc, 0xED, &[0x64, 0x03, 0x12, 0x81]); // LCD_POWER_SEQ
    lcd_cmd_data(spi, cs, dc, 0xE8, &[0x85, 0x00, 0x78]); // LCD_DTCA
    lcd_cmd_data(spi, cs, dc, 0xCB, &[0x39, 0x2C, 0x00, 0x34, 0x02]); // LCD_POWERA
    lcd_cmd_data(spi, cs, dc, 0xF7, &[0x20]); // LCD_PRC
    lcd_cmd_data(spi, cs, dc, 0xEA, &[0x00, 0x00]); // LCD_DTCB

    lcd_cmd_data(spi, cs, dc, 0xB1, &[0x00, 0x1B]); // LCD_FRMCTR1

    // First DFC write
    lcd_cmd_data(spi, cs, dc, 0xB6, &[0x0A, 0xA2]); // LCD_DFC

    lcd_cmd_data(spi, cs, dc, 0xC0, &[0x10]); // LCD_POWER1
    lcd_cmd_data(spi, cs, dc, 0xC1, &[0x10]); // LCD_POWER2
    lcd_cmd_data(spi, cs, dc, 0xC5, &[0x45, 0x15]); // LCD_VCOM1
    lcd_cmd_data(spi, cs, dc, 0xC7, &[0x90]); // LCD_VCOM2

    lcd_cmd_data(spi, cs, dc, 0x36, &[0xC8]); // LCD_MAC
    lcd_cmd_data(spi, cs, dc, 0xF2, &[0x00]); // LCD_3GAMMA_EN

    // This is one of the key RGB-interface commands
    lcd_cmd_data(spi, cs, dc, 0xB0, &[0xC2]); // LCD_RGB_INTERFACE

    // Second DFC write - also important for LTDC/RGB mode
    lcd_cmd_data(spi, cs, dc, 0xB6, &[0x0A, 0xA7, 0x27, 0x04]); // LCD_DFC

    // Column address set: 0..239
    lcd_cmd_data(spi, cs, dc, 0x2A, &[0x00, 0x00, 0x00, 0xEF]); // LCD_COLUMN_ADDR

    // Page address set: 0..319
    lcd_cmd_data(spi, cs, dc, 0x2B, &[0x00, 0x00, 0x01, 0x3F]); // LCD_PAGE_ADDR

    // Interface control - important for RGB interface
    lcd_cmd_data(spi, cs, dc, 0xF6, &[0x01, 0x00, 0x06]); // LCD_INTERFACE

    lcd_cmd(spi, cs, dc, 0x2C); // LCD_GRAM
    Timer::after_millis(200).await;

    lcd_cmd_data(spi, cs, dc, 0x26, &[0x01]); // LCD_GAMMA

    lcd_cmd_data(
        spi,
        cs,
        dc,
        0xE0, // LCD_PGAMMA
        &[
            0x0F, 0x29, 0x24, 0x0C, 0x0E, 0x09, 0x4E, 0x78, 0x3C, 0x09, 0x13, 0x05, 0x17, 0x11,
            0x00,
        ],
    );

    lcd_cmd_data(
        spi,
        cs,
        dc,
        0xE1, // LCD_NGAMMA
        &[
            0x00, 0x16, 0x1B, 0x04, 0x11, 0x07, 0x31, 0x33, 0x42, 0x05, 0x0C, 0x0A, 0x28, 0x2F,
            0x0F,
        ],
    );

    lcd_cmd(spi, cs, dc, 0x11); // LCD_SLEEP_OUT
    Timer::after_millis(200).await;

    lcd_cmd(spi, cs, dc, 0x29); // LCD_DISPLAY_ON

    // GRAM start writing
    lcd_cmd(spi, cs, dc, 0x2C); // LCD_GRAM
}

fn lcd_cmd(spi: &mut impl SpiBus, cs: &mut impl OutputPin, dc: &mut impl OutputPin, cmd: u8) {
    cs.set_low().unwrap();
    dc.set_low().unwrap();
    spi.write(&[cmd]).unwrap();
    //spi.blocking_write(&[cmd]).unwrap();
    cs.set_high().unwrap();
}

fn lcd_data(spi: &mut impl SpiBus, cs: &mut impl OutputPin, dc: &mut impl OutputPin, data: &[u8]) {
    cs.set_low().unwrap();
    dc.set_high().unwrap();
    spi.write(data).unwrap();
    cs.set_high().unwrap();
}

fn lcd_cmd_data(
    spi: &mut impl SpiBus,
    cs: &mut impl OutputPin,
    dc: &mut impl OutputPin,
    cmd: u8,
    data: &[u8],
) {
    lcd_cmd(spi, cs, dc, cmd);
    lcd_data(spi, cs, dc, data);
}
