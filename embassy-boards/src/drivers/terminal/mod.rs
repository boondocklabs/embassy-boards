//! Ratatui Terminal Backend

extern crate alloc;
use core::convert::Infallible;

use alloc::vec::Vec;
use embassy_stm32::{
    dma2d::{self, Buffer2D, ColorConfig, Dma2d},
    ltdc::{self, Ltdc},
    peripherals::{DMA2D, LTDC},
};
use embedded_graphics::mono_font::MonoFont;

use embedded_graphics_unicodefonts::mono_8x13_atlas;
use ratatui::{
    layout::{Position, Size},
    prelude::Backend,
    style::Color,
};

use crate::display::{
    fonts::FontCache,
    texture::{Texture, TextureAtlas},
};

#[allow(unused)]
pub struct RenderServer {
    dma2d: Dma2d<'static, DMA2D>,
    ltdc: Ltdc<'static, LTDC, ltdc::DSI>,
    cache: FontCache<'static, 2500>,
    font_buffer: Buffer2D,
    fg: Buffer2D,
    bg: Buffer2D,
    font: MonoFont<'static>,

    frame_count: usize,

    cursor_position: Position,
    size: Size,
}

impl RenderServer {
    pub async fn new(
        mut dma2d: Dma2d<'static, DMA2D>,
        ltdc: Ltdc<'static, LTDC, ltdc::DSI>,
        fg: Buffer2D,
        bg: Buffer2D,
        font_texture: Texture,
    ) -> Self {
        dma2d
            .fill(&bg.region(0, 0, fg.width, fg.height), 0)
            .await
            .unwrap();

        dma2d
            .fill(&fg.region(0, 0, fg.width, fg.height), 0x700000ff)
            .await
            .unwrap();

        #[cfg(feature = "defmt")]
        defmt::info!("{}", font_texture);

        let font = mono_8x13_atlas();

        let w = fg.width / font.character_size.width as u16;
        let h = fg.height / font.character_size.height as u16;
        let size = Size::new(w, h);

        // Get a DMA2D buffer descriptor over the whole font atlas
        let font_buffer = font_texture.dma2d_buffer();

        let font_atlas = TextureAtlas::<2500>::new(
            font_texture,
            font.character_size.width,
            font.character_size.height,
        );

        let cache = FontCache::new(font_atlas, font);

        let mut fg_color = ColorConfig::default();
        fg_color.pixel_format = dma2d::PixelFormat::A8;
        fg_color.alpha_mode = dma2d::AlphaMode::NoModify;

        let mut cfg = ColorConfig::default();
        cfg.alpha_mode = dma2d::AlphaMode::NoModify;

        dma2d.set_color_config(dma2d::BufferKind::Foreground, &fg_color);
        dma2d.set_color_config(dma2d::BufferKind::Background, &cfg);
        dma2d.set_color_config(dma2d::BufferKind::Output, &cfg);

        Self {
            dma2d,
            ltdc,
            cache,
            font_buffer,
            fg,
            bg,
            font,
            cursor_position: Position { x: 0, y: 0 },
            size,
            frame_count: 0,
        }
    }

    async fn render_glyph(&mut self, ch: char, position: Position, fg: Color, bg: Color) {
        let a = self.cache.glyph(ch);
        let glyph_region =
            self.font_buffer
                .region(a.x as u16, a.y as u16, a.width as u16, a.height as u16);

        let x = position.x * self.font.character_size.width as u16;
        let y = position.y * self.font.character_size.height as u16;
        let w = self.font.character_size.width as u16;
        let h = self.font.character_size.height as u16;

        // Get a back buffer region
        let output_region = self.fg.region(x, y, w, h);

        let fg_color = match fg {
            Color::Reset => 0xffffffff,
            _ => {
                let rgb = ratatui_color_to_rgb(fg);
                (0xff as u32) << 24 | (rgb.0 as u32) << 16 | (rgb.1 as u32) << 8 | rgb.2 as u32
            }
        };

        let bg_color = match bg {
            Color::Reset => 0x00000000,
            _ => {
                let rgb = ratatui_color_to_rgb(bg);
                (0xff as u32) << 24 | (rgb.0 as u32) << 16 | (rgb.1 as u32) << 8 | rgb.2 as u32
            }
        };

        let mut color_config = ColorConfig::default();
        color_config.pixel_format = dma2d::PixelFormat::A8;
        color_config.alpha_mode = dma2d::AlphaMode::Replace((bg_color >> 24) as u8);

        self.dma2d
            .set_color_config(dma2d::BufferKind::Background, &color_config);

        self.dma2d
            .blit(
                &glyph_region,
                &output_region,
                Some(fg_color),
                Some(bg_color),
            )
            .await
            .unwrap();
    }

    pub async fn set_background(&mut self, data: Vec<u8>) {
        let buf = Buffer2D::new(
            data.as_ptr() as *mut u8,
            dma2d::PixelFormat::Argb8888,
            480,
            480,
            800,
        );

        let input_region = buf.region(0, 0, 480, 800);
        let output_region = self.bg.region(0, 0, 480, 800);

        let mut fg_color = ColorConfig::default();
        fg_color.pixel_format = dma2d::PixelFormat::Argb8888;
        fg_color.alpha_mode = dma2d::AlphaMode::NoModify;

        self.dma2d
            .set_color_config(dma2d::BufferKind::Foreground, &fg_color);

        self.dma2d
            .blit(&input_region, &output_region, None, None)
            .await
            .unwrap();
    }
}

impl Backend for RenderServer {
    type Error = Infallible;

    async fn draw<'a, I>(&mut self, content: I) -> Result<(), Self::Error>
    where
        I: Iterator<Item = (u16, u16, &'a ratatui::buffer::Cell)>,
    {
        for (x, y, cell) in content {
            for ch in cell.symbol().chars() {
                self.render_glyph(ch, Position { x, y }, cell.fg, cell.bg)
                    .await;
            }
        }

        Ok(())
    }

    fn hide_cursor(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn show_cursor(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn get_cursor_position(&mut self) -> Result<ratatui::prelude::Position, Self::Error> {
        Ok(self.cursor_position)
    }

    fn set_cursor_position<P: Into<ratatui::prelude::Position>>(
        &mut self,
        position: P,
    ) -> Result<(), Self::Error> {
        self.cursor_position = position.into();
        Ok(())
    }

    async fn clear(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn clear_region(
        &mut self,
        _clear_type: ratatui::prelude::backend::ClearType,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn size(&self) -> Result<ratatui::prelude::Size, Self::Error> {
        Ok(self.size)
    }

    fn window_size(&mut self) -> Result<ratatui::prelude::backend::WindowSize, Self::Error> {
        todo!()
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        /*
        self.frame_count += 1;

        self.ltdc
            .set_buffer(
                embassy_stm32::ltdc::LtdcLayer::Layer1,
                self.active_fb().ptr.as_ptr() as *const _,
            )
            .await
            .unwrap();

        self.dma2d
            .copy(self.active_fb(), self.back_fb())
            .await
            .unwrap();

        self.ltdc
            .set_buffer(
                embassy_stm32::ltdc::LtdcLayer::Layer1,
                self.active_fb().ptr.as_ptr() as *const _,
            )
            .await
            .unwrap();
            */

        Ok(())
    }
}

#[inline(always)]
pub fn ratatui_color_to_rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Reset => (0, 0, 0),

        // Standard ANSI (approximate xterm palette)
        Color::Black => (0, 0, 0),
        Color::Red => (205, 0, 0),
        Color::Green => (0, 205, 0),
        Color::Yellow => (205, 205, 0),
        Color::Blue => (0, 0, 238),
        Color::Magenta => (205, 0, 205),
        Color::Cyan => (0, 205, 205),
        Color::Gray => (229, 229, 229),

        // Bright variants
        Color::DarkGray => (127, 127, 127),
        Color::LightRed => (255, 0, 0),
        Color::LightGreen => (0, 255, 0),
        Color::LightYellow => (255, 255, 0),
        Color::LightBlue => (92, 92, 255),
        Color::LightMagenta => (255, 0, 255),
        Color::LightCyan => (0, 255, 255),
        Color::White => (255, 255, 255),

        // Truecolor
        Color::Rgb(r, g, b) => (r, g, b),

        // 256-color palette
        Color::Indexed(i) => indexed_to_rgb(i),
    }
}

fn indexed_to_rgb(i: u8) -> (u8, u8, u8) {
    match i {
        // 0–15: system colors (reuse above mapping if you want)
        0 => (0, 0, 0),
        1 => (128, 0, 0),
        2 => (0, 128, 0),
        3 => (128, 128, 0),
        4 => (0, 0, 128),
        5 => (128, 0, 128),
        6 => (0, 128, 128),
        7 => (192, 192, 192),
        8 => (128, 128, 128),
        9 => (255, 0, 0),
        10 => (0, 255, 0),
        11 => (255, 255, 0),
        12 => (0, 0, 255),
        13 => (255, 0, 255),
        14 => (0, 255, 255),
        15 => (255, 255, 255),

        // 16–231: 6×6×6 color cube
        16..=231 => {
            let i = i - 16;
            let r = (i / 36) % 6;
            let g = (i / 6) % 6;
            let b = i % 6;

            fn scale(v: u8) -> u8 {
                if v == 0 { 0 } else { 55 + v * 40 }
            }

            (scale(r), scale(g), scale(b))
        }

        // 232–255: grayscale ramp
        232..=255 => {
            let gray = 8 + (i - 232) * 10;
            (gray, gray, gray)
        }
    }
}
