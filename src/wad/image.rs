use read::WadRead;
use math::Vec2;
use sdl2::pixels::PixelFormatEnum;
use std::path::Path;
use std::vec::Vec;
use types::WadTextureHeader;
use std::borrow::Cow;
use std::fmt::{self, Debug, Display};

pub struct Image {
    width: usize,
    height: usize,
    x_offset: isize,
    y_offset: isize,
    pixels: Vec<u16>,
}

#[derive(Debug)]
pub struct ImageError(Cow<'static, str>);
impl Image {
    pub fn new(width: usize, height: usize) -> Result<Image, ImageError> {
        if width >= 4096 || height >= 4096 {
            return Err(format!("image too large {}x{}", width, height).into());
        }
        Ok(Image {
            width: width,
            height: height,
            x_offset: 0,
            y_offset: 0,
            pixels: vec![0xff00; width * height],
        })
    }

    pub fn new_from_header(header: &WadTextureHeader) -> Result<Image, ImageError> {
        Image::new(header.width as usize, header.height as usize)
    }

    pub fn from_buffer(buffer: &[u8]) -> Result<Image, ImageError> {
        let mut reader = buffer;
        let width = try!(reader.wad_read::<u16>().to_err("missing width")) as usize;
        let height = try!(reader.wad_read::<u16>().to_err("missing height")) as usize;
        if width >= 4096 || height >= 4096 {
            return Err(format!("image too large {}x{}", width, height).into());
        }

        let x_offset = try!(reader.wad_read::<i16>().to_err("missing x offset")) as isize;
        let y_offset = try!(reader.wad_read::<i16>().to_err("missing y offset")) as isize;

        let mut pixels = vec![!0; width * height];

        // Process each column of the image.
        for i_column in 0..width {
            // Each column is defined as a number of vertical `runs' which are
            // defined starting at `offset' in the buffer.
            let offset = try!(reader.wad_read::<u32>()
                                    .to_err_with(|| {
                                        format!("unfinished column {}, {}x{}",
                                                i_column,
                                                width,
                                                height)
                                    })) as isize;
            if offset >= buffer.len() as isize {
                return Err(format!("invalid column offset in {}, offset={}, size={}",
                                   i_column,
                                   offset,
                                   buffer.len())
                               .into());
            }
            let mut source = buffer[offset as usize..].iter();
            let mut i_run = 0;
            loop {
                // The first byte contains the vertical coordinate of the run's
                // start.
                let row_start = *try!(source.next()
                                            .to_err_with(|| {
                                                format!("unfinshed column {}, run {}",
                                                        i_column,
                                                        i_run)
                                            })) as usize;

                // The special value of 255 means this is the last run in the
                // column, so move on to the next one.
                if row_start == 255 {
                    break;
                }

                // The second byte is the length of this run. Skip an additional
                // byte which is ignored for some reason.
                let run_length = *try!(source.next()
                                             .to_err_with(|| {
                                                 format!("missing run length: column {}, run {}",
                                                         i_column,
                                                         i_run)
                                             })) as usize;

                // Check that the run fits in the image.
                if row_start + run_length > height {
                    return Err(format!("run too big: column {}, run {} ({} +{}), size {}x{}",
                                       i_column,
                                       i_run,
                                       row_start,
                                       run_length,
                                       width,
                                       height)
                                   .into());
                }

                // An ignored padding byte.
                try!(source.next()
                           .to_err_with(|| {
                               format!("missing padding byte 1: column {}, run {}", i_column, i_run)
                           }));

                // Iterator to the beginning of the run in `pixels`. Guaranteed to be in bounds
                // by the check above.
                let mut destination = pixels[row_start * width + i_column..]
                                          .chunks_mut(width)
                                          .map(|row| &mut row[0])
                                          .take(run_length);

                // Copy the bytes from source to destination, but first check there's enough of
                // those left.
                if source.size_hint().0 < run_length {
                    return Err(format!("source underrun: column {}, run {} ({}, +{}), bytes \
                                        left {}",
                                       i_column,
                                       i_run,
                                       row_start,
                                       run_length,
                                       source.size_hint().0)
                                   .into());
                }
                for dest_pixel in &mut destination {
                    *dest_pixel = *source.next().expect("missing pixel despite check") as u16;
                }

                // And another ignored byte after the run.
                try!(source.next()
                           .to_err_with(|| {
                               format!("missing padding byte 2: column {}, run {}", i_column, i_run)
                           }));
                i_run += 1;
            }
        }

        Ok(Image {
            width: width,
            height: height,
            x_offset: x_offset,
            y_offset: y_offset,
            pixels: pixels,
        })
    }

    pub fn blit(&mut self, source: &Image, offset: Vec2<isize>, ignore_transparency: bool) {
        // Figure out the region in source which is not out of bounds when
        // copied into self.
        let y_start = if offset[1] < 0 {
            (-offset[1]) as usize
        } else {
            0
        };
        let x_start = if offset[0] < 0 {
            (-offset[0]) as usize
        } else {
            0
        };
        let y_end = if self.height as isize > source.height as isize + offset[1] {
            source.height
        } else {
            (self.height as isize - offset[1]) as usize
        };
        let x_end = if self.width as isize > source.width as isize + offset[0] {
            source.width
        } else {
            (self.width as isize - offset[0]) as usize
        };

        let src_pitch = source.width as usize;
        let dest_pitch = self.width as usize;
        let copy_width = x_end - x_start;
        let copy_height = (y_end - y_start) as usize;
        let (x_start, y_start) = (x_start as usize, y_start as usize);

        let src_rows = &source.pixels[x_start + y_start * src_pitch..];
        let src_rows = src_rows.chunks(src_pitch)
                               .take(copy_height)
                               .map(|row| &row[..copy_width]);

        let dest_rows = &mut self.pixels[(x_start as isize + offset[0]) as usize +
                                         (y_start as isize + offset[1]) as usize * dest_pitch..];
        let dest_rows = dest_rows.chunks_mut(dest_pitch)
                                 .take(copy_height)
                                 .map(|row| &mut row[..copy_width]);

        if ignore_transparency {
            // If we don't care about transparency we can copy row by row.
            for (dest_row, source_row) in dest_rows.zip(src_rows) {
                for (dest_pixel, &source_pixel) in dest_row.iter_mut().zip(source_row.iter()) {
                    *dest_pixel = source_pixel;
                }
            }
        } else {
            // Only copy pixels whose high bits are not set.
            for (dest_row, source_row) in dest_rows.zip(src_rows) {
                for (dest_pixel, &source_pixel) in dest_row.iter_mut().zip(source_row.iter()) {
                    // `Blending' is a simple copy/no copy, but using bit ops we can avoid
                    // branching.
                    let blend = 0u16.wrapping_sub(source_pixel >> 15);
                    *dest_pixel = (source_pixel & !blend) | (*dest_pixel & blend);
                }
            }
        }
    }

    pub fn x_offset(&self) -> isize {
        self.x_offset
    }
    pub fn y_offset(&self) -> isize {
        self.y_offset
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn size(&self) -> Vec2<usize> {
        Vec2::new(self.width, self.height)
    }

    pub fn num_pixels(&self) -> usize {
        self.pixels.len()
    }

    pub fn pixels(&self) -> &[u16] {
        &self.pixels
    }

    pub fn into_pixels(self) -> Vec<u16> {
        self.pixels
    }

    pub fn save_bmp<P: AsRef<Path> + Debug>(&self,
                                            palette: &[[u8; 3]; 256],
                                            path: &P)
                                            -> Result<(), ImageError> {
        use sdl2::surface::Surface;
        let mut pixels = vec![0u8; 3 * self.width * self.height];
        for (index, pixel) in self.pixels.iter().enumerate() {
            let pixel = palette[(pixel & 0xff) as usize];
            pixels[index * 3] = pixel[2];
            pixels[index * 3 + 1] = pixel[1];
            pixels[index * 3 + 2] = pixel[0];
        }
        let surface = try!(Surface::from_data(&mut pixels[..],
                                              self.width as u32,
                                              self.height as u32,
                                              self.width as u32 * 3,
                                              PixelFormatEnum::BGR24)
                               .to_err("failed to create surface"));
        surface.save_bmp(path).to_err_with(|| format!("failed to save bmp {:?}", path))
    }
}

trait ToError {
    type Success;

    fn to_err(self, &'static str) -> Result<Self::Success, ImageError>;
    fn to_err_with<F: FnOnce() -> String>(self, with: F) -> Result<Self::Success, ImageError>;
}

impl<S, E> ToError for Result<S, E> {
    type Success = S;

    fn to_err(self, message: &'static str) -> Result<Self::Success, ImageError> {
        self.map_err(|_| ImageError(Cow::Borrowed(message)))
    }

    fn to_err_with<F: FnOnce() -> String>(self, with: F) -> Result<Self::Success, ImageError> {
        self.map_err(|_| ImageError(Cow::Owned(with())))
    }
}

impl<S> ToError for Option<S> {
    type Success = S;

    fn to_err(self, message: &'static str) -> Result<Self::Success, ImageError> {
        self.ok_or(ImageError(Cow::Borrowed(message)))
    }

    fn to_err_with<F: FnOnce() -> String>(self, with: F) -> Result<Self::Success, ImageError> {
        self.ok_or_else(|| ImageError(Cow::Owned(with())))
    }
}

impl From<String> for ImageError {
    fn from(value: String) -> ImageError {
        ImageError(Cow::Owned(value))
    }
}

impl From<&'static str> for ImageError {
    fn from(value: &'static str) -> ImageError {
        ImageError(Cow::Borrowed(value))
    }
}

impl Display for ImageError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}
