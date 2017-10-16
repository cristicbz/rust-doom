use super::types::WadTextureHeader;
use byteorder::{ReadBytesExt, LittleEndian};
use math::Vec2;
use std::vec::Vec;

pub mod error {
    error_chain!{}
}

use self::error::{Result, ResultExt, ErrorKind};


pub const MAX_IMAGE_SIZE: usize = 4096;


pub struct Image {
    width: usize,
    height: usize,
    x_offset: isize,
    y_offset: isize,
    pixels: Vec<u16>,
}

impl Image {
    pub fn new(width: usize, height: usize) -> Result<Image> {
        ensure!(
            width <= MAX_IMAGE_SIZE && height <= MAX_IMAGE_SIZE,
            "image too large {}x{}",
            width,
            height
        );
        Ok(Image {
            width: width,
            height: height,
            x_offset: 0,
            y_offset: 0,
            pixels: vec![0xff00; width * height],
        })
    }

    pub fn new_from_header(header: &WadTextureHeader) -> Result<Image> {
        Image::new(header.width as usize, header.height as usize)
    }

    #[cfg_attr(feature = "cargo-clippy", allow(needless_range_loop))]
    pub fn from_buffer(buffer: &[u8]) -> Result<Image> {
        let mut reader = buffer;
        let width = reader.read_u16::<LittleEndian>().chain_err(
            || "missing width",
        )? as usize;
        let height = reader.read_u16::<LittleEndian>().chain_err(
            || "missing height",
        )? as usize;
        ensure!(
            width <= MAX_IMAGE_SIZE && height <= MAX_IMAGE_SIZE,
            "image too large {}x{}",
            width,
            height
        );

        let x_offset = reader.read_i16::<LittleEndian>().chain_err(
            || "missing x offset",
        )? as isize;
        let y_offset = reader.read_i16::<LittleEndian>().chain_err(
            || "missing y offset",
        )? as isize;

        let mut pixels = vec![!0; width * height];

        // Process each column of the image.
        for i_column in 0..width {
            // Each column is defined as a number of vertical `runs' which are
            // defined starting at `offset' in the buffer.
            let offset = reader.read_u32::<LittleEndian>().chain_err(|| {
                format!("unfinished column {}, {}x{}", i_column, width, height)
            })? as isize;
            ensure!(
                offset < buffer.len() as isize,
                "invalid column offset in {}, offset={}, size={}",
                i_column,
                offset,
                buffer.len()
            );
            let mut source = buffer[offset as usize..].iter();
            let mut i_run = 0;
            loop {
                // The first byte contains the vertical coordinate of the run's
                // start.
                let row_start = *source.next().ok_or_else(|| {
                    ErrorKind::from(format!("unfinshed column {}, run {}", i_column, i_run))
                })? as usize;

                // The special value of 255 means this is the last run in the
                // column, so move on to the next one.
                if row_start == 255 {
                    break;
                }

                // The second byte is the length of this run. Skip an additional
                // byte which is ignored for some reason.
                let run_length = *source.next().ok_or_else(|| {
                    ErrorKind::from(format!(
                        "missing run length: column {}, run {}",
                        i_column,
                        i_run
                    ))
                })? as usize;

                // Check that the run fits in the image.
                ensure!(
                    row_start + run_length <= height,
                    "run too big: column {}, run {} ({} +{}), size {}x{}",
                    i_column,
                    i_run,
                    row_start,
                    run_length,
                    width,
                    height
                );

                // An ignored padding byte.
                ensure!(
                    source.next().is_some(),
                    "missing padding byte 1: column {}, run {}",
                    i_column,
                    i_run
                );

                // Iterator to the beginning of the run in `pixels`. Guaranteed to be in bounds
                // by the check above.
                let mut destination = pixels[row_start * width + i_column..]
                    .chunks_mut(width)
                    .map(|row| &mut row[0])
                    .take(run_length);

                // Copy the bytes from source to destination, but first check there's enough of
                // those left.
                ensure!(
                    source.size_hint().0 >= run_length,
                    "source underrun: column {}, run {} ({}, +{}), bytes  left {}",
                    i_column,
                    i_run,
                    row_start,
                    run_length,
                    source.size_hint().0
                );
                for dest_pixel in &mut destination {
                    *dest_pixel = u16::from(*source.next().expect("missing pixel despite check"));
                }

                // And another ignored byte after the run.
                ensure!(
                    source.next().is_some(),
                    "missing padding byte 2: column {}, run {}",
                    i_column,
                    i_run
                );
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
        let src_rows = src_rows.chunks(src_pitch).take(copy_height).map(|row| {
            &row[..copy_width]
        });

        let dest_rows = &mut self.pixels[(x_start as isize + offset[0]) as usize +
                                             (y_start as isize + offset[1]) as usize *
                                                 dest_pitch..];
        let dest_rows = dest_rows.chunks_mut(dest_pitch).take(copy_height).map(
            |row| {
                &mut row[..copy_width]
            },
        );

        if ignore_transparency {
            // If we don't care about transparency we can copy row by row.
            for (dest_row, source_row) in dest_rows.zip(src_rows) {
                dest_row.copy_from_slice(source_row);
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

    //pub fn save_bmp<P: AsRef<Path> + Debug>(
    //    &self,
    //    palette: &[[u8; 3]; 256],
    //    path: &P,
    //) -> Result<()> {
    //    let mut pixels = vec![0u8; 3 * self.width * self.height];
    //    for (index, pixel) in self.pixels.iter().enumerate() {
    //        let pixel = palette[(pixel & 0xff) as usize];
    //        pixels[index * 3] = pixel[2];
    //        pixels[index * 3 + 1] = pixel[1];
    //        pixels[index * 3 + 2] = pixel[0];
    //    }
    //    let surface = Surface::from_data(
    //        &mut pixels[..],
    //        self.width as u32,
    //        self.height as u32,
    //        self.width as u32 * 3,
    //        PixelFormatEnum::BGR24,
    //    ).map_err(|error| {
    //        ErrorKind::from(format!("failed to create surface: {}", error))
    //    })?;

    //    surface.save_bmp(path).map_err(|error| {
    //        ErrorKind::from(format!("failed to save bmp to {:?}: {}", path, error))
    //    })?;

    //    Ok(())
    //}
}
