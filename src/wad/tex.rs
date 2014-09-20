use std::io::{BufReader, Reader, SeekSet};
use std::{str, mem};
use super::types::*;
use super::Archive;
use super::util::read_binary;

static PNAMES_LUMP_NAME: &'static [u8, ..8] =
    &[b'P', b'N', b'A', b'M', b'E', b'S', b'\0', b'\0'];

static TEXTURE_LUMP_NAMES: &'static [[u8, ..8]] =
    &[[b'T', b'E', b'X', b'T', b'U', b'R', b'E', b'1'],
      [b'T', b'E', b'X', b'T', b'U', b'R', b'E', b'2']];


pub struct TextureDirectory {
    //textures: HashMap<Vec<u8>, Image>,
    patches: Vec<(WadName, Option<Image>)>,
}

pub struct Image {
    width: uint,
    height: uint,
    x_offset: int,
    y_offset: int,
    pixels: Vec<i16>,
}

impl Image {
    pub fn new(width: uint, height: uint) -> Image{
        Image { width: width,
                height: height,
                x_offset: 0,
                y_offset: 0,
                pixels: Vec::from_elem(width * height, 0) }
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
                    let pixel = reader.read_u8().unwrap() as i16;
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
                if source_pixel >= 0 || overwrite {
                    *self_pixel = source_pixel;
                }
            }
        }
    }

    pub fn get_x_offset(&self) -> int { self.x_offset }
    pub fn get_y_offset(&self) -> int { self.y_offset }
}

macro_rules! io_try(
    ($e:expr) => (try!($e.map_err(|e| String::from_str(e.desc))))
)

impl TextureDirectory {
    pub fn from_archive(wad: &mut Archive) -> Result<TextureDirectory, String> {
        info!("Reading texture directory...");

        // Read patches.
        let lump_buffer = wad.read_lump_by_name(PNAMES_LUMP_NAME);
        let mut lump = BufReader::new(lump_buffer.as_slice());

        let num_patches = io_try!(lump.read_le_u32()) as uint;
        info!("  {:4} patches", num_patches);
        let mut patches = Vec::with_capacity(num_patches);
        let mut missing_patches = 0u;
        for i_patch in range(0, num_patches) {
            let name = read_binary::<WadName, _>(&mut lump);
            let patch = wad.get_lump_index(&name).map(|index| {
                let patch_buffer = wad.read_lump(index);
                Image::from_buffer(patch_buffer.as_slice())
            });
            if patch.is_none() { missing_patches += 1; }
            patches.push((name, patch));
        }
        let patches = patches;

        // Read textures.
        for lump_name in TEXTURE_LUMP_NAMES.iter() {
            let lump_index = match wad.get_lump_index(lump_name) {
                Some(i) => i,
                None => continue
            };

            let lump_buffer = wad.read_lump(lump_index);
            let mut lump = BufReader::new(lump_buffer.as_slice());

            let num_textures = io_try!(lump.read_le_u32()) as uint;
            info!("  {:4} textures in {}", num_textures,
                  str::from_utf8(lump_name).unwrap());

            let mut offsets = BufReader::new({
                let begin = io_try!(lump.tell()) as uint;
                let size = num_textures * mem::size_of::<u32>();
                lump_buffer.slice(begin, begin + size)
            });

            for i_texture in range(0, num_textures) {
                io_try!(lump.seek(io_try!(offsets.read_le_u32()) as i64,
                                  SeekSet));
                let header = read_binary::<WadTextureHeader, _>(&mut lump);
                let mut image = Image::new_from_header(&header);

                for i_patch in range(0, header.num_patches) {
                    let pref = read_binary::<WadTexturePatchRef, _>(&mut lump);
                    match patches[pref.patch as uint] {
                        (_, Some(ref patch)) => {
                            image.blit(patch,
                                       pref.origin_x as int,
                                       pref.origin_y as int,
                                       i_patch == 0);
                        },
                        (ref patch_name, None) => {
                            fail!("Texture {} uses missing patch {}.",
                                  str::from_utf8(header.name).unwrap(),
                                  str::from_utf8(patch_name).unwrap());
                        }
                    }
                }
            }
        }

        warn!("{} missing patches from PNAMES.", missing_patches);
        Ok(TextureDirectory { patches: patches })
    }
}
