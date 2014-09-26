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

        let mut pixels = Vec::from_elem(width * height, -1);
        for i_column in range(0, width as int) { unsafe {
            let offset = reader.read_le_u32().unwrap() as int;
            let mut s_ptr = buffer.as_ptr().offset(offset);
            loop {
                let row_start = *s_ptr as int; s_ptr = s_ptr.offset(1);
                if row_start == 255 { break }
                let run_length = *s_ptr as int; s_ptr = s_ptr.offset(2);
                let s_end = s_ptr.offset(run_length);
                let mut d_ptr = pixels.as_mut_ptr()
                                      .offset(row_start * width as int
                                              + i_column);
                while s_ptr < s_end {
                    *d_ptr = *s_ptr as u16;
                    d_ptr = d_ptr.offset(width as int);
                    s_ptr = s_ptr.offset(1);
                }
                s_ptr = s_ptr.offset(1);
            }
        }}
        let pixels = pixels;

        Image { width: width,
                height: height,
                x_offset: x_offset,
                y_offset: y_offset,
                pixels: pixels }
    }

    pub fn blit(&mut self, source: &Image, x_offset: int, y_offset: int,
                overwrite: bool) {
        let y_start = if y_offset < 0 { -y_offset as uint } else { 0 };
        let x_start = if x_offset < 0 { -x_offset as uint } else { 0 };
        let y_end = if self.height as int > source.height as int + y_offset {
            source.height
        } else {
            self.height - y_offset as uint
        };
        let x_end = if self.width as int > source.width as int + x_offset {
            source.width
        } else {
            self.width - x_offset as uint
        };

        if overwrite {
            for source_y in range(y_start, y_end) {
                let self_y = source_y as int + y_offset;
                for source_x in range(x_start, x_end) {
                    let self_x = source_x as int + x_offset;

                    let (self_x, self_y) = (self_x as uint, self_y as uint);
                    let self_index = (self_x + self_y * self.width) as int;
                    let source_index = (source_x + source_y * source.width)
                        as int;

                    unsafe {
                        let source_pixel = *source.pixels.as_ptr()
                                                         .offset(source_index);
                        *self.pixels.as_mut_ptr().offset(self_index) =
                            source_pixel;
                    }
                }
            }
        } else {
            for source_y in range(y_start, y_end) {
                let self_y = source_y as int + y_offset;
                for source_x in range(x_start, x_end) {
                    let self_x = source_x as int + x_offset;

                    let (self_x, self_y) = (self_x as uint, self_y as uint);
                    let self_index = (self_x + self_y * self.width) as int;
                    let source_index = (source_x + source_y * source.width)
                        as int;

                    unsafe {
                        let source_pixel = *source.pixels.as_ptr()
                                                         .offset(source_index);
                        if source_pixel & 0xff00 == 0 {
                            *self.pixels.as_mut_ptr().offset(self_index) =
                                source_pixel;
                        }
                    }
                }
            }
        }
    }

    pub fn get_x_offset(&self) -> int { self.x_offset }
    pub fn get_y_offset(&self) -> int { self.y_offset }

    pub fn get_width(&self) -> uint { self.width }
    pub fn get_height(&self) -> uint { self.height }

    pub fn get_pixels<'a>(&'a self) -> &'a [u16] { self.pixels.as_slice() }
}

