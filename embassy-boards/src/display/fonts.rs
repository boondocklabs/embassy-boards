//! Font glyph rendering/cache

use embedded_graphics::geometry::OriginDimensions;
use embedded_graphics::image::ImageDrawable;
use embedded_graphics::image::{ImageDrawableExt, ImageRaw, SubImage};
use embedded_graphics::mono_font::MonoFont;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::{DrawTarget, Point};
use embedded_graphics::primitives::Rectangle;
use hashbrown::HashMap;

use super::texture::{TextureAtlas, TextureRegion};

pub struct FontIndex {}

pub type FontColor = embedded_graphics::pixelcolor::Gray8;

/// Font cache atlas
pub struct FontCache<'a, const N: usize> {
    atlas: TextureAtlas<N>,

    /// Font
    font: MonoFont<'a>,

    index: HashMap<char, usize>,
}

impl<'a, const N: usize> FontCache<'a, N> {
    pub fn new(atlas: TextureAtlas<N>, font: MonoFont<'a>) -> Self {
        Self {
            atlas,
            font,
            index: HashMap::new(),
        }
    }

    pub fn glyph(&mut self, c: char) -> TextureRegion<'_> {
        if let Some(index) = self.index.get(&c) {
            self.atlas.region_for_index(*index)
        } else {
            let image = Self::font_glyph(&self.font, c);

            let mut region = self.atlas.alloc().expect("Out of font cache space");
            self.index.insert(c, region.index);

            region.clear(BinaryColor::Off);

            // Draw the font glyph into the region
            image.draw(&mut region);

            region
        }
    }

    fn font_glyph<'b>(font: &'b MonoFont, c: char) -> SubImage<'b, ImageRaw<'b, BinaryColor>> {
        let glyphs_per_row = font.image.size().width / font.character_size.width;

        let glyph_index = font.glyph_mapping.index(c) as u32;
        let row = glyph_index / glyphs_per_row;

        // Top left corner of character, in pixels
        let char_x = (glyph_index - (row * glyphs_per_row)) * font.character_size.width;
        let char_y = row * font.character_size.height;

        font.image.sub_image(&Rectangle::new(
            Point::new(char_x as i32, char_y as i32),
            font.character_size,
        ))
    }
}
