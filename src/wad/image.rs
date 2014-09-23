use std::vec::Vec;
use std::io::{BufReader, SeekSet};
use super::types::*;


pub struct Image {
    width: uint,
    height: uint,
    x_offset: int,
    y_offset: int,
    pixels: Vec<u16>,
}


macro_rules! io_try(
    ($e:expr) => (try!($e.map_err(|e| String::from_str(e.desc))))
)


impl Image {
    pub fn new(width: uint, height: uint) -> Image{
        Image { width: width,
                height: height,
                x_offset: 0,
                y_offset: 0,
                pixels: Vec::from_elem(width * height, 0xff00) }
    }

    pub fn new_from_header(header: &WadTextureHeader) -> Image {
        Image::new(header.width as uint, header.height as uint)
    }

    pub fn from_buffer(buffer: &[u8]) -> Image {
        let mut reader = BufReader::new(buffer);

        let width = reader.read_le_u16().unwrap() as uint;
        let height = reader.read_le_u16().unwrap() as uint;
        let x_offset = reader.read_le_i16().unwrap() as int;
        let y_offset = reader.read_le_i16().unwrap() as int;

        // This allocation isn't strictly necessary.
        let mut column_offsets = Vec::with_capacity(width);
        for i_column in range(0, width) {
            column_offsets.push(reader.read_le_u32().unwrap() as i64);
        }
        let column_offsets = column_offsets;

        let mut pixels = Vec::from_elem(width * height, -1);
        for i_column in range(0, width) {
            reader.seek(column_offsets[i_column], SeekSet).unwrap();
            loop {
                let row_start = reader.read_u8().unwrap() as uint;
                if row_start == 255 { break }
                let run_length = reader.read_u8().unwrap() as uint;
                reader.read_u8().unwrap();  // Ignore first byte.
                for i_run in range(0, run_length) {
                    let index = (i_run + row_start) * width + i_column;
                    let pixel = reader.read_u8().unwrap() as u16;
                    *pixels.get_mut(index) = pixel;
                }
                reader.read_u8().unwrap();  // Ignore last byte.
            }
        }
        let pixels = pixels;

        Image { width: width,
                height: height,
                x_offset: x_offset,
                y_offset: y_offset,
                pixels: pixels }
    }

    pub fn blit(&mut self, source: &Image, x_offset: int, y_offset: int,
                overwrite: bool) {
        for source_y in range(0, source.height) {
            let self_y = source_y as int + y_offset;
            if self_y < 0 || self_y >= self.height as int { continue; }

            for source_x in range(0, source.width) {
                let self_x = source_x as int + x_offset;
                if self_x < 0 || self_x >= self.width as int { continue; }

                let (self_x, self_y) = (self_x as uint, self_y as uint);
                let source_index = source_x + source_y * source.width;
                let self_index = self_x + self_y * self.width;

                let self_pixel = self.pixels.get_mut(self_index);
                let source_pixel = source.pixels[source_index];
                if source_pixel & 0xff00 == 0 || overwrite {
                    *self_pixel = source_pixel;
                }
            }
        }
    }

    pub fn get_x_offset(&self) -> int { self.x_offset }
    pub fn get_y_offset(&self) -> int { self.y_offset }

    pub fn get_width(&self) -> uint { self.width }
    pub fn get_height(&self) -> uint { self.height }

    pub fn get_pixels<'a>(&'a self) -> &'a [u16] { self.pixels.as_slice() }
    //pub fn with_u8_pixels<T>(&self, fun: |&[u8]| -> T) -> T {
    //    unsafe {
    //        raw::buf_as_slice(
    //            self.pixels.as_ptr() as *const u8,
    //            self.pixels.len() * 2,
    //            fun)
    //    }
    //}
}

