//! Graphics textures

use core::{convert::Infallible, marker::PhantomData, ptr::NonNull};

use embassy_stm32::dma2d::{self, Buffer2D, PixelFormat};
use embedded_graphics::{
    Pixel,
    pixelcolor::{BinaryColor, Gray8},
    prelude::{DrawTarget, IntoStorage, OriginDimensions, Size},
};

/// DMA2D texture buffer.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Texture {
    data: NonNull<u8>,
    pub width: u32,
    pub height: u32,
    pub stride_bytes: usize,
    pub format: PixelFormat,
}

impl Texture {
    pub const unsafe fn new(data: *mut u8, width: u32, height: u32, format: PixelFormat) -> Self {
        let stride_bytes = Self::calc_stride_bytes(width, format)
            .expect("pixel format requires non-byte-aligned or unsupported stride");
        Self {
            data: unsafe { NonNull::new_unchecked(data) },
            width,
            height,
            stride_bytes,
            format,
        }
    }

    pub fn dma2d_buffer(&self) -> Buffer2D {
        Buffer2D::new(
            self.data.as_ptr(),
            dma2d::PixelFormat::A8,
            self.width as u16,
            self.width as u16,
            self.height as u16,
        )
    }

    pub const fn calc_stride_bytes(width: u32, format: PixelFormat) -> Option<usize> {
        let width = width as usize;
        match format {
            PixelFormat::A8 | PixelFormat::L8 => Some(width),
            PixelFormat::Rgb565 => Some(width * 2),
            PixelFormat::Argb8888 => Some(width * 4),
            PixelFormat::A4 => Some(width.div_ceil(2)),
            _ => None,
        }
    }

    pub fn bytes_per_pixel(&self) -> Option<usize> {
        Self::bytes_per_pixel_for(self.format)
    }

    pub fn bytes_per_pixel_for(format: PixelFormat) -> Option<usize> {
        match format {
            PixelFormat::A8 | PixelFormat::L8 => Some(1),
            PixelFormat::Rgb565 => Some(2),
            PixelFormat::Argb8888 => Some(4),
            PixelFormat::A4 => None,
            _ => None,
        }
    }

    pub fn required_len(&self) -> usize {
        self.stride_bytes * self.height as usize
    }

    pub fn ptr_at(&self, x: u32, y: u32) -> *const u8 {
        debug_assert!(x < self.width);
        debug_assert!(y < self.height);

        let row = y as usize * self.stride_bytes;
        let col = match self.format {
            PixelFormat::A4 => (x as usize) / 2,
            PixelFormat::A8 | PixelFormat::L8 => x as usize,
            PixelFormat::Rgb565 => x as usize * 2,
            PixelFormat::Argb8888 => x as usize * 4,
            _ => unreachable!("unsupported pixel format"),
        };

        unsafe { self.data.as_ptr().add(row + col) }
    }

    pub fn mut_ptr_at(&mut self, x: u32, y: u32) -> *mut u8 {
        debug_assert!(x < self.width);
        debug_assert!(y < self.height);

        let row = y as usize * self.stride_bytes;
        let col = match self.format {
            PixelFormat::A4 => (x as usize) / 2,
            PixelFormat::A8 | PixelFormat::L8 => x as usize,
            PixelFormat::Rgb565 => x as usize * 2,
            PixelFormat::Argb8888 => x as usize * 4,
            _ => unreachable!("unsupported pixel format"),
        };

        unsafe { self.data.as_ptr().add(row + col) }
    }
}

/// A region view into a Texture.
#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TextureRegion<'a> {
    pub base: NonNull<u8>,
    pub parent_stride_bytes: usize,
    pub format: PixelFormat,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub index: usize,
    _phantom: PhantomData<&'a mut [u8]>,
}

impl<'a> TextureRegion<'a> {
    #[inline]
    fn offset_of(&self, x: u32, y: u32) -> usize {
        debug_assert!(x < self.width);
        debug_assert!(y < self.height);

        let px = self.x as usize + x as usize;
        let py = self.y as usize + y as usize;

        let col = match self.format {
            PixelFormat::A4 => px / 2,
            PixelFormat::A8 | PixelFormat::L8 => px,
            PixelFormat::Rgb565 => px * 2,
            PixelFormat::Argb8888 => px * 4,
            _ => unreachable!("unsupported pixel format"),
        };

        py * self.parent_stride_bytes + col
    }

    #[inline]
    pub fn put_u8(&mut self, x: u32, y: u32, value: u8) {
        debug_assert!(matches!(self.format, PixelFormat::A8 | PixelFormat::L8));
        let offset = self.offset_of(x, y);

        unsafe {
            self.base.as_ptr().add(offset).write(value);
        }
    }

    #[inline]
    pub fn get_u8(&self, x: u32, y: u32) -> u8 {
        debug_assert!(matches!(self.format, PixelFormat::A8 | PixelFormat::L8));
        let offset = self.offset_of(x, y);

        unsafe { self.base.as_ptr().add(offset).read() }
    }

    #[cfg(feature = "defmt")]
    pub fn debug(&self) {
        const MAX_W: usize = 512; // set to your max expected width

        for y in 0..self.height {
            let mut buf = [0u8; MAX_W];

            for x in 0..self.width as usize {
                buf[x] = match self.get_u8(x as u32, y) {
                    0 => b'.',
                    1..=127 => b'+',
                    _ => b'#',
                };
            }

            let s = core::str::from_utf8(&buf[..self.width as usize]).unwrap();
            defmt::info!("{}", s);
        }
    }
}

pub struct TextureAtlas<const N: usize> {
    pub texture: Texture,
    pub cell_width: u32,
    pub cell_height: u32,
    cols: u32,
    rows: u32,
    used: [bool; N],
}

impl<'a, const N: usize> TextureAtlas<N> {
    pub fn new(texture: Texture, cell_width: u32, cell_height: u32) -> Self {
        assert!(cell_width > 0);
        assert!(cell_height > 0);

        let cols = texture.width / cell_width;
        let rows = texture.height / cell_height;

        assert!(cols > 0);
        assert!(rows > 0);
        assert!((cols as usize) * (rows as usize) >= N);

        Self {
            texture,
            cell_width,
            cell_height,
            cols,
            rows,
            used: [false; N],
        }
    }

    pub fn capacity(&self) -> usize {
        (self.cols * self.rows) as usize
    }

    pub fn alloc(&mut self) -> Option<TextureRegion<'_>> {
        let cap = self.capacity();
        for index in 0..cap {
            if !self.used[index] {
                self.used[index] = true;
                return Some(self.region_for_index(index));
            }
        }
        None
    }

    pub fn free(&mut self, region: TextureRegion<'_>) -> bool {
        if let Some(index) = self.index_for_region(&region) {
            if self.used[index] {
                self.used[index] = false;
                return true;
            }
        }
        false
    }

    pub fn region_for_index<'b>(&'b mut self, index: usize) -> TextureRegion<'b> {
        let col = (index as u32) % self.cols;
        let row = (index as u32) / self.cols;

        TextureRegion {
            base: NonNull::new(self.texture.data.as_ptr()).unwrap(),
            parent_stride_bytes: self.texture.stride_bytes,
            format: self.texture.format,
            x: col * self.cell_width,
            y: row * self.cell_height,
            width: self.cell_width,
            height: self.cell_height,
            index,
            _phantom: PhantomData,
        }
    }

    pub fn index_for_region(&self, region: &TextureRegion<'_>) -> Option<usize> {
        if region.width != self.cell_width || region.height != self.cell_height {
            return None;
        }
        if region.x % self.cell_width != 0 || region.y % self.cell_height != 0 {
            return None;
        }

        let col = region.x / self.cell_width;
        let row = region.y / self.cell_height;

        if col >= self.cols || row >= self.rows {
            return None;
        }

        Some((row * self.cols + col) as usize)
    }
}

impl<'a> OriginDimensions for TextureRegion<'a> {
    fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }
}

impl<'a> DrawTarget for TextureRegion<'a> {
    type Color = BinaryColor;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            if point.x < 0 || point.y < 0 {
                continue;
            }

            let x = point.x as u32;
            let y = point.y as u32;

            if x >= self.width || y >= self.height {
                continue;
            }

            let a8: Gray8 = color.into();

            self.put_u8(x, y, a8.into_storage());
        }

        Ok(())
    }
}
