//! Crate to generate CAPTCHAs.
//!
//! <img src="https://github.com/daniel-e/captcha/raw/master/doc/captcha3.png"> &nbsp;
//! <img src="https://github.com/daniel-e/captcha/raw/master/doc/captcha2.png">
//!
//! The crate offers two ways to create CAPTCHAs. The first way is the most easiest one. Just one
//! line of code is necessary to create a random CAPTCHA in of a given category. The category can be
//! either `Easy`, `Medium` or `Hard`. The size of the CAPTCHA is 220x120 pixels and the number of
//! characters is randomly selected between 4 and 6 letters (inclusive).
//!
//! ```
//! # extern crate captcha;
//! use captcha::{gen, Difficulty};
//!
//! # fn main() {
//! gen(Difficulty::Easy).as_png();
//! # }
//! ```
//!
//! To be more flexible you can build CAPTCHAs manually as well.
//!
//! ```
//! # extern crate captcha;
//! use captcha::Captcha;
//! use captcha::filters::{Noise, Wave, Dots};
//!
//! # fn main() {
//! Captcha::new()
//!     .add_chars(5)
//!     .apply_filter(Noise::new(0.4))
//!     .apply_filter(Wave::new(2.0, 20.0).horizontal())
//!     .apply_filter(Wave::new(2.0, 20.0).vertical())
//!     .view(220, 120)
//!     .apply_filter(Dots::new(15))
//!     .as_png();
//! # }
//! ```

// TODO rotate characters
// TODO resize characters
// TODO overlap characters

extern crate image;
extern crate rand;
extern crate serde_json;
extern crate base64;
extern crate lodepng;

pub mod filters;
mod samples;
mod images;
mod fonts;

pub use samples::{gen, Difficulty};

use filters::Filter;
use images::{Image, Pixl};
use fonts::{Default, Font};

use std::path::Path;
use std::io::Result;
use rand::{thread_rng, Rng};
use std::cmp::{min, max};

/// Represents the area which contains text in a CAPTCHA.
#[derive(Clone, Debug)]
pub struct Geometry {
    /// The minimum x coordinate of the area which contains text (inclusive).
    pub left: u32,
    /// The maximum x coordinate of the area which contains text (inclusive).
    pub right: u32,
    /// The minimum y coordinate of the area which contains text (inclusive).
    pub top: u32,
    /// The maximum y coordinate of the area which contains text (inclusive).
    pub bottom: u32
}

/// Used to build a CAPTCHA step by step.
pub struct Captcha {
    img: Image,
    font: Box<Font>,
    text_area: Geometry,
    chars: Vec<char>
}

impl Captcha {
    /// Returns an empty CAPTCHA.
    pub fn new() -> Captcha {
        // TODO fixed width + height
        let w = 400;
        let h = 300;
        Captcha {
            img      : Image::new(w, h),
            font     : Box::new(Default::new()),
            text_area: Geometry { left: w / 4, right: w / 4, top: h / 2, bottom: h / 2 },
            chars    : vec![]
        }
    }

    /// Applies the filter `f` to the CAPTCHA.
    ///
    /// This method is used to add noise, grids, etc or to transform the shape of the CAPTCHA.
    pub fn apply_filter<F: Filter>(&mut self, f: F) -> &mut Self {
        f.apply(&mut self.img);
        self
        // TODO support other fonts
    }

    /// Sets another font that is used for the characters.
    ///
    /// Calling this method does not have an effect on the font of the characters which have already
    /// been added to the CAPTCHA. The new font is only applied to the characters which are written
    /// to the CAPTCHA after this method is called.
    pub fn set_font<F: Font + 'static>(&mut self, f: F) -> &mut Self {
        self.font = Box::new(f);
        self
        // TODO support other fonts
    }

    /// Saves the CAPTCHA to a image file.
    ///
    /// The format that is written is determined from the filename's extension. On error `Err` is
    /// returned.
    pub fn save(&self, p: &Path) -> Result<()> { self.img.save(p) }

    fn random_char_as_image(&self) -> Option<(char, Image)> {
        let mut rng = thread_rng();
        match rng.choose(&self.font.chars()) {
            None    => None,
            Some(c) => {
                match self.font.png(c.clone()) {
                    None    => None,
                    Some(p) => match Image::from_png(p) {
                        None    => None,
                        Some(i) => Some((c.clone(), i))
                    }
                }
            }
        }
    }

    /// Adds a random character using the current font.
    pub fn add_char(&mut self) -> &mut Self {
        match self.random_char_as_image() {
            Some((c, i)) => {
                let x = self.text_area.right;
                let y = (self.text_area.bottom + self.text_area.top) / 2 - i.height() / 2;
                self.img.add_image(x, y, &i);

                self.text_area.top    = min(self.text_area.top, y);
                self.text_area.right  = x + i.width() - 1;
                self.text_area.bottom = max(self.text_area.bottom, y + i.height() - 1);
                self.chars.push(c);
            },
            _ => { }
        }

        self
        // TODO automatically resize if many characters are added
    }

    /// Adds a red box to the CAPTCHA representing the area which contains text.
    pub fn add_text_area(&mut self) -> &mut Self {
        for y in self.text_area.top..self.text_area.bottom {
            self.img.put_pixel(self.text_area.left, y, Pixl::red());
            self.img.put_pixel(self.text_area.right, y, Pixl::red());
        }
        for x in self.text_area.left..self.text_area.right {
            self.img.put_pixel(x, self.text_area.top, Pixl::red());
            self.img.put_pixel(x, self.text_area.bottom, Pixl::red());
        }
        self
    }

    /// Returns the geometry of the area which contains text in the CAPTCHA.
    pub fn text_area(&self) -> Geometry {
        self.text_area.clone()
    }

    /// Crops the CAPTCHA to the given geometry.
    pub fn extract(&mut self, area: Geometry) -> &mut Self {
        // TODO rename the method
        // TODO adjust the text area
        let w = area.right - area.left + 1;
        let h = area.bottom - area.top + 1;
        let mut i = Image::new(w, h);
        for (y, iy) in (area.top..area.bottom + 1).zip(0..h + 1) {
            for (x, ix) in (area.left..area.right + 1).zip(0..w + 1) {
                i.put_pixel(ix, iy, self.img.get_pixel(x, y));
            }
        }
        self.img = i;
        self
    }

    /// Crops the CAPTCHA to the given width and height with the text centered withing this
    /// box.
    pub fn view(&mut self, w: u32, h: u32) -> &mut Self {
        let mut a = self.text_area();
        a.left   = (a.right + a.left) / 2 - w / 2;
        a.right  = a.left + w;
        a.top    = (a.bottom + a.top) / 2 - h / 2;
        a.bottom = a.top + h;
        self.extract(a);
        // TODO update text area
        // TODO what happens if w or h are too small
        self
    }

    /// Returns the characters added to this CAPTCHA.
    pub fn chars(&self) -> Vec<char> {
        self.chars.clone()
    }

    /// Adds the given number of random characters to the CAPTCHA using the current font.
    pub fn add_chars(&mut self, n: u32) -> &mut Self {
        for _ in 0..n {
            self.add_char();
        }
        self
    }

    /// Returns the CAPTCHA as a png image.
    ///
    /// Returns `None` on error.
    pub fn as_png(&self) -> Option<Vec<u8>> {
        self.img.as_png()
    }
}

#[cfg(test)]
mod tests {
    use Captcha;
    use filters::{Noise, Grid};
    use fonts::Default;

    use std::path::Path;

    #[test]
    fn it_works() {
        let mut c = Captcha::new();
        c.set_font(Default::new())
            .add_char()
            .add_char()
            .add_char()
            .apply_filter(Noise::new(0.1))
            .apply_filter(Grid::new(20, 10))
            .add_text_area();

        let a = c.text_area();
        c.extract(a).save(Path::new("/tmp/captcha.png")).expect("save failed");
        c.as_png().expect("no png");
    }
}
