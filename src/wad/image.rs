use base::vec_from_elem;
use gfx::Texture;
use gl;
use std::ptr::copy_nonoverlapping_memory;
use std::vec::Vec;
use std::io::Read;
use super::types::WadTextureHeader;


pub struct Image {
    width: usize,
    height: usize,
    x_offset: isize,
    y_offset: isize,
    pixels: Vec<u16>,
}


macro_rules! io_try(
    ($e:expr) => (try!($e.map_err(|e| String::from_str(e.desc))))
);


impl Image {
    pub fn new(width: usize, height: usize) -> Image {
        Image { width: width,
                height: height,
                x_offset: 0,
                y_offset: 0,
                pixels: vec_from_elem(width * height, 0xff00) }
    }

    pub fn new_from_header(header: &WadTextureHeader) -> Image {
        Image::new(header.width as usize, header.height as usize)
    }

    pub fn from_buffer(buffer: &[u8]) -> Image {
        let mut reader = buffer;
        let width = reader.read_le_u16().unwrap() as usize;
        let height = reader.read_le_u16().unwrap() as usize;
        let x_offset = reader.read_le_i16().unwrap() as isize;
        let y_offset = reader.read_le_i16().unwrap() as isize;

        let mut pixels = vec![-1; width * height];
        for i_column in 0 .. width {
            let column_offset = reader.read_le_u32().unwrap() as usize;
            let mut column = buffer[column_offset..].iter().cloned();
            loop {
                let row_start = match column.next().unwrap() {
                    255 => break,
                    x => x,
                };

                let run_length = column.next().unwrap();
                column.next().unwrap();
                let mut dest_idx = row_start as usize * width + i_column;
                for i in 0..run_length {
                    pixels[dest_idx] = column.next().unwrap() as u16;
                    dest_idx += width;
                }
                column.next().unwrap();
            }
        }
        let pixels = pixels;
        //// Sorry for the messy/unsafe code, but the array bounds checks in this
        //// tight loop make it 6x slower.
        //for i_column in range(0, width as isize) { unsafe {
        //    // Each column is defined as a number of vertical `runs' which are
        //    // defined starting at `offset' in the buffer.
        //    let offset = buffer.read_le_u32().unwrap() as isize;
        //    let mut src_ptr = buffer.as_ptr().offset(offset);
        //    'this_column: loop {
        //        // The first byte contains the vertical coordinate of the run's
        //        // start.
        //        let row_start = *src_ptr as isize; src_ptr = src_ptr.offset(1);

        //        // Pointer to the beginning of the run in `pixels'.
        //        let mut dest_ptr = pixels
        //            .as_mut_ptr().offset(row_start * width as isize + i_column);

        //        // The special value of 255 means this is the last run in the
        //        // column, so move on to the next one.
        //        if row_start == 255 { break 'this_column; }

        //        // The second byte is the length of this run. Skip an additional
        //        // byte which is ignored for some reason.
        //        let run_length = *src_ptr as isize; src_ptr = src_ptr.offset(2);

        //        let src_end = src_ptr.offset(run_length);  // Ptr to end of run.
        //        while src_ptr < src_end {
        //            // Copy one byte converting it into an opaque pixels (high
        //            // bits are zero) and advance the pointers.
        //            *dest_ptr = *src_ptr as u16;
        //            dest_ptr = dest_ptr.offset(width as isize);
        //            src_ptr = src_ptr.offset(1);
        //        }
        //        // And another ignored byte after the run.
        //        src_ptr = src_ptr.offset(1);
        //    }
        //}}
        //let pixels = pixels;

        Image { width: width,
                height: height,
                x_offset: x_offset,
                y_offset: y_offset,
                pixels: pixels }
    }

    pub fn blit(&mut self, source: &Image, x_offset: isize, y_offset: isize,
                ignore_transparency: bool) {
        // Figure out the region in source which is not out of bounds when
        // copied into self.
        let y_start = if y_offset < 0 { (-y_offset) as usize } else { 0 };
        let x_start = if x_offset < 0 { (-x_offset) as usize } else { 0 };
        let y_end = if self.height as isize > source.height as isize + y_offset {
            source.height
        } else {
            (self.height as isize - y_offset) as usize
        };
        let x_end = if self.width as isize > source.width as isize + x_offset {
            source.width
        } else {
            (self.width as isize - x_offset) as usize
        };

        if ignore_transparency {
            // If we don't care about transparency we can memcpy row by row.
            let src_pitch = source.width as isize;
            let dest_pitch = self.width as isize;
            let copy_width = x_end - x_start;
            let copy_height = (y_end - y_start) as isize;
            let (x_start, y_start) = (x_start as isize, y_start as isize);
            unsafe {
                let mut src_ptr = source.pixels.as_ptr().offset(
                    x_start + y_start * src_pitch);
                let mut dest_ptr = self.pixels.as_mut_ptr().offset(
                    x_start + x_offset + (y_start + y_offset) * dest_pitch);

                let src_end = src_ptr.offset(copy_height * src_pitch);
                while src_ptr < src_end {
                    copy_nonoverlapping_memory(dest_ptr, src_ptr, copy_width);
                    src_ptr = src_ptr.offset(src_pitch);
                    dest_ptr = dest_ptr.offset(dest_pitch);
                }
            }
        } else {
            // Only copy pixels whose high bits are not set.
            let dest_ptr = self.pixels.as_mut_ptr();
            let src_ptr = source.pixels.as_ptr();
            for src_y in range(y_start, y_end) {
                let dest_y = (src_y as isize + y_offset) as usize;
                for src_x in range(x_start, x_end) {
                    let dest_x = (src_x as isize + x_offset) as usize;

                    let (dest_x, dest_y) = (dest_x as usize, dest_y as usize);
                    let dest_index = (dest_x + dest_y * self.width) as isize;
                    let src_index = (src_x + src_y * source.width) as isize;

                    unsafe {
                        // `Blending' is a simple copy/no copy, but using bit
                        // ops we can avoid branching.
                        let src_pixel = *src_ptr.offset(src_index);
                        let dest_pixel = dest_ptr.offset(dest_index);
                        let blend = (0.wrapping_sub(src_pixel >> 15)) as u16;
                        *dest_pixel = (src_pixel & !blend) |
                                      (*dest_pixel & blend);
                    }
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

    pub fn num_pixels(&self) -> usize { self.pixels.len() }

    pub fn get_pixels(&self) -> &[u16] { &self.pixels }

}

