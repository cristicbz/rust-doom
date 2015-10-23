use byteorder::{LittleEndian, ByteOrder, ReadBytesExt};
use gfx::Texture;
use gl;
use math::Vec2;
use sdl2::pixels::PixelFormatEnum;
use std::path::Path;
use std::vec::Vec;
use types::WadTextureHeader;

pub struct Image {
    width: usize,
    height: usize,
    x_offset: isize,
    y_offset: isize,
    pixels: Vec<u16>,
}

impl Image {
    pub fn new(width: usize, height: usize) -> Image {
        Image {
            width: width,
            height: height,
            x_offset: 0,
            y_offset: 0,
            pixels: vec![0xff00; width * height],
        }
    }

    pub fn new_from_header(header: &WadTextureHeader) -> Image {
        Image::new(header.width as usize, header.height as usize)
    }

    pub fn from_buffer(buffer: &[u8]) -> Image {
        let mut reader = buffer;
        let width = reader.read_u16::<LittleEndian>().unwrap() as usize;
        let height = reader.read_u16::<LittleEndian>().unwrap() as usize;
        let x_offset = reader.read_i16::<LittleEndian>().unwrap() as isize;
        let y_offset = reader.read_i16::<LittleEndian>().unwrap() as isize;

        let mut pixels = vec![!0; width * height];

        // Process each column of the image.
        for i_column in 0 .. width {
            // Each column is defined as a number of vertical `runs' which are
            // defined starting at `offset' in the buffer.
            let offset = reader.read_u32::<LittleEndian>().unwrap() as isize;
            let mut source = buffer[offset as usize..].iter();
            loop {
                // The first byte contains the vertical coordinate of the run's
                // start.
                let row_start = *source.next().unwrap() as usize;

                // The special value of 255 means this is the last run in the
                // column, so move on to the next one.
                if row_start == 255 {
                    break;
                }

                // The second byte is the length of this run. Skip an additional
                // byte which is ignored for some reason.
                let run_length = *source.next().unwrap() as usize;
                source.next().unwrap();

                // Iterator to the beginning of the run in `pixels`.
                let mut destination = pixels[row_start * width + i_column..]
                    .chunks_mut(width)
                    .map(|row| &mut row[0])
                    .take(run_length);

                while let Some(dest_pixel) = destination.next() {
                    *dest_pixel = *source.next().unwrap() as u16;
                }
                // And another ignored byte after the run.
                source.next().unwrap();
            }
        }

        Image {
            width: width,
            height: height,
            x_offset: x_offset,
            y_offset: y_offset,
            pixels: pixels
        }
    }

    pub fn blit(&mut self, source: &Image, offset: Vec2<isize>, ignore_transparency: bool) {
        // Figure out the region in source which is not out of bounds when
        // copied into self.
        let y_start = if offset[1] < 0 { (-offset[1]) as usize } else { 0 };
        let x_start = if offset[0] < 0 { (-offset[0]) as usize } else { 0 };
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

        let source_rows = source.pixels[x_start + y_start * src_pitch..]
                                .chunks(src_pitch)
                                .take(copy_height)
                                .map(|row| &row[..copy_width]);
        let dest_rows = self.pixels[(x_start as isize + offset[0]) as usize
                                    + (y_start as isize + offset[1]) as usize * dest_pitch..]
                            .chunks_mut(dest_pitch)
                            .take(copy_height)
                            .map(|row| &mut row[..copy_width]);

        if ignore_transparency {
            // If we don't care about transparency we can copy row by row.
            for (dest_row, source_row) in dest_rows.zip(source_rows) {
                for (dest_pixel, &source_pixel) in dest_row.iter_mut().zip(source_row.iter()) {
                    *dest_pixel = source_pixel;
                }
            }
        } else {
            // Only copy pixels whose high bits are not set.
            for (dest_row, source_row) in dest_rows.zip(source_rows) {
                for (dest_pixel, &source_pixel) in dest_row.iter_mut().zip(source_row.iter()) {
                    // `Blending' is a simple copy/no copy, but using bit ops we can avoid
                    // branching.
                    let blend = 0u16.wrapping_sub(source_pixel >> 15);
                    *dest_pixel = (source_pixel & !blend) | (*dest_pixel & blend);
                }
            }
        }
    }

    pub fn to_texture(&self) -> Texture {
        let mut tex = Texture::new(gl::TEXTURE_2D);
        tex.bind(gl::TEXTURE0);
        tex.set_filters_nearest()
           .data_rg_u8(0, self.width, self.height, &self.pixels)
           .unbind(gl::TEXTURE0);
        tex
    }

    pub fn x_offset(&self) -> isize { self.x_offset }
    pub fn y_offset(&self) -> isize { self.y_offset }

    pub fn width(&self) -> usize { self.width }
    pub fn height(&self) -> usize { self.height }

    pub fn size(&self) -> Vec2<usize> { Vec2::new(self.width, self.height) }

    pub fn num_pixels(&self) -> usize { self.pixels.len() }

    pub fn pixels(&self) -> &[u16] { &self.pixels }

    pub fn save_bmp<P: AsRef<Path>>(&self, palette: &[[u8; 3]; 256], path: &P) {
        use ::sdl2::surface::Surface;
        let mut pixels = vec![0u8; 3 * self.width * self.height];
        for (index, pixel) in self.pixels.iter().enumerate() {
            let pixel = palette[(pixel & 0xff) as usize];
            pixels[index * 3] = pixel[2];
            pixels[index * 3 + 1] = pixel[1];
            pixels[index * 3 + 2] = pixel[0];
        }
        Surface::from_data(&mut pixels[..],
                           self.width as u32,
                           self.height as u32,
                           self.width as u32 * 3,
                           PixelFormatEnum::BGR24)
            .unwrap()
            .save_bmp(path)
            .unwrap();
    }
}

