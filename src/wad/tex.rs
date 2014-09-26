use std::collections::HashMap;
use std::io::{BufReader, Reader, SeekSet};
use std::{str, mem};

use super::Archive;
use super::image::Image;
use super::types::*;
use super::util::{read_binary, name_toupper};

use texture::Texture;

use gl;
use numvec::{Vec2, Vec2f};


pub type Palette = [[u8, ..3], ..256];
pub type Colormap = [u8, ..256];
pub type Flat = Vec<u8>;

pub struct Bounds {
    pub pos: Vec2f,
    pub size: Vec2f,
}

pub struct TextureDirectory {
    textures: HashMap<Vec<u8>, Image>,
    patches: Vec<(WadName, Option<Image>)>,
    palettes: Vec<Palette>,
    colormaps: Vec<Colormap>,
    flats: HashMap<Vec<u8>, Flat>,
}

macro_rules! io_try(
    ($e:expr) => (try!($e.map_err(|e| String::from_str(e.desc))))
)

impl TextureDirectory {
    pub fn from_archive(wad: &mut Archive) -> Result<TextureDirectory, String> {
        info!("Reading texture directory...");
        // Read palettes & colormaps.
        let palettes = wad.read_lump_by_name(PLAYPAL_LUMP_NAME);
        let colormaps = wad.read_lump_by_name(COLORMAP_LUMP_NAME);
        info!("  {:4} palettes", palettes.len());
        info!("  {:4} colormaps", colormaps.len());

        // Read patches.
        let patches = try!(read_patches(wad));
        info!("  {:4} patches", patches.len());

        // Read textures.
        let mut textures = HashMap::new();
        for lump_name in TEXTURE_LUMP_NAMES.iter() {
            let lump_index = match wad.get_lump_index(lump_name) {
                Some(i) => i,
                None => {
                    info!("     0 textures in {}",
                          str::from_utf8(lump_name));
                    continue
                }
            };
            let num_textures = try!(
                read_textures(wad.read_lump(lump_index).as_slice(),
                              patches.as_slice(), &mut textures));
            info!("  {:4} textures in {}",
                  num_textures, str::from_utf8(lump_name));
        }
        let textures = textures;

        // Read flats.
        let flats = try!(read_flats(wad));
        info!("  {:4} flats", flats.len());


        Ok(TextureDirectory {
            patches: patches,
            textures: textures,
            palettes: palettes,
            colormaps: colormaps,
            flats: flats,
        })
    }

    pub fn get_texture<'a>(&'a self, name: &[u8]) -> Option<&'a Image> {
        self.textures.find(&name_toupper(name))
    }
    pub fn get_flat<'a>(&'a self, name: &[u8]) -> Option<&'a Flat> {
        self.flats.find(&name_toupper(name))
    }

    pub fn num_patches(&self) -> uint { self.patches.len() }
    pub fn get_patch<'a>(&'a self, index: uint) -> Option<&'a Image> {
        self.patches[index].1.as_ref()
    }

    pub fn num_palettes(&self) -> uint { self.palettes.len() }
    pub fn get_palette<'a>(&'a self, index: uint) -> &'a Palette {
        &self.palettes[index]
    }

    pub fn num_colormaps(&self) -> uint { self.colormaps.len() }
    pub fn get_colormap<'a>(&'a self, index: uint) -> &'a Colormap {
        &self.colormaps[index]
    }

    pub fn build_palette_texture(&self,
                                 palette: uint,
                                 colormap_start: uint,
                                 colormap_end: uint) -> Texture {
        let num_colormaps = colormap_end - colormap_start;
        let mut data = Vec::from_elem(256 * num_colormaps * 3, 0u8);
        let palette = &self.palettes[palette];
        for i_colormap in range(colormap_start, colormap_end) {
            for i_color in range(0, 256) {
                let rgb = &palette[self.colormaps[i_colormap][i_color] as uint];
                *data.get_mut(0 + i_color * 3 + i_colormap * 768) = rgb[0];
                *data.get_mut(1 + i_color * 3 + i_colormap * 768) = rgb[1];
                *data.get_mut(2 + i_color * 3 + i_colormap * 768) = rgb[2];
            }
        }

        let mut palette_tex = Texture::new(gl::TEXTURE_2D);
        palette_tex.bind(gl::TEXTURE0);
        palette_tex
            .set_filters_nearest()
            .data_rgb_u8(0, 256, num_colormaps, data.as_slice())
            .unbind(gl::TEXTURE0);
        palette_tex
    }

    pub fn build_colormap_texture(&self) -> Texture {
        let mut colormap_tex = Texture::new(gl::TEXTURE_2D);
        colormap_tex.bind(gl::TEXTURE0);
        colormap_tex
            .set_filters_nearest()
            .data_red_u8(0, 256, self.colormaps.len(),
                         self.colormaps.as_slice())
            .unbind(gl::TEXTURE0);
        colormap_tex
    }

    pub fn build_wall_atlas<'a, T: Iterator<&'a [u8]>>(&self, names_iter: T)
            -> (Texture, HashMap<Vec<u8>, Bounds>) {
        let images = names_iter.map(|n|
                                    (n, self.get_texture(n).expect(format!(
                                        "Wall texture '{}' missing.",
                                        &str::from_utf8(n)).as_slice()
                                    )))
                               .collect::<Vec<(&'a [u8], &Image)>>();
        assert!(images.len() > 0, "No images in wall atlas.");

        fn img_bound(x_offset: uint, y_offset: uint, img: &Image) -> Bounds {
            Bounds { pos: Vec2::new(x_offset as f32, y_offset as f32),
                     size: Vec2::new(img.get_width() as f32,
                                     img.get_height() as f32) }
        }

        let mut bounds = Vec::with_capacity(images.len());
        let mut size = images.iter().map(|t| t.1.get_width()).max().unwrap();
        size = next_pow2(size);
        let max_size = 4096;
        let mut optional_atlas = None;
        loop {
            let mut atlas = Image::new(size, size);
            let mut x_offset = 0;
            let mut y_offset = 0;
            let mut failed = false;
            let mut max_height = 0;
            for &(_, image) in images.iter() {
                let (width, height) = (image.get_width(), image.get_height());
                if height > max_height { max_height = height; }
                if x_offset + width > size {
                    x_offset = 0;
                    y_offset += max_height;
                    max_height = 0;
                }
                if y_offset + height > size {
                    failed = true;
                    break;
                }
                bounds.push(img_bound(x_offset, y_offset, image));
                atlas.blit(image, x_offset as int, y_offset as int, true);
                x_offset += width;
            }

            if failed {
                size *= 2;
                if size > max_size { break; }
                bounds.clear();
            } else {
                optional_atlas = Some(atlas);
                break;
            }
        }
        let atlas = optional_atlas.expect("Could not fit wall atlas");
        let size = size;

        assert!(bounds.len() == images.len());
        let mut bound_map = HashMap::with_capacity(images.len());
        for i in range(0, images.len()) {
            bound_map.insert(name_toupper(images[i].0), bounds[i]);
        }
        drop(bounds);

        let mut tex = Texture::new(gl::TEXTURE_2D);
        tex.bind(gl::TEXTURE0);
        tex.set_filters_nearest()
           .data_rg_u8(0, size, size, atlas.get_pixels())
           .unbind(gl::TEXTURE0);

        info!("Wall texture atlas size: {}x{}", size, size);
        (tex, bound_map)

    }

    pub fn build_flat_atlas<'a, T: Iterator<&'a [u8]>>(&self,
                                                       num_names: uint,
                                                       mut names_iter: T)
            -> (Texture, HashMap<Vec<u8>, Vec2f>) {

        let width = next_pow2((num_names as f64).sqrt().ceil() as uint * 64);
        let flats_per_row = width / 64;

        let num_rows = (num_names as f64 / flats_per_row as f64).ceil() as uint;
        let height = next_pow2(num_rows * 64);

        let mut offsets = HashMap::with_capacity(num_names);
        let mut data = Vec::from_elem(width * height, 255u8);
        let (mut row, mut column) = (0, 0);
        info!("Flat atlas size: {}x{} ({}, {})", width, height, flats_per_row,
                                                 num_rows);
        for _ in range(0, num_names) {
            let name = names_iter.next().expect("Not enough flats.");
            let flat = self.get_flat(name).expect("Unknown flat.");
            let x_offset = column * 64;
            let y_offset = row * 64;
            offsets.insert(name_toupper(name),
                           Vec2::new(x_offset as f32 / width as f32,
                                     y_offset as f32 / height as f32));

            for y in range(0, 64) {
                for x in range(0, 64) {
                    *data.get_mut(x_offset + x + (y + y_offset) * width)
                        = flat[x + y * 64];
                }
            }

            column += 1;
            if column == flats_per_row {
                column = 0;
                row += 1;
            }
        }

        let mut tex = Texture::new(gl::TEXTURE_2D);
        tex.bind(gl::TEXTURE0);
        tex.set_filters_nearest()
           .data_red_u8(0, width, height, data.as_slice())
           .unbind(gl::TEXTURE0);

        (tex, offsets)
    }

}


fn next_pow2(x: uint) -> uint {
    let mut pow2 = 1;
    while pow2 < x { pow2 *= 2; }
    pow2
}


static PNAMES_LUMP_NAME: &'static [u8, ..8] =
    &[b'P', b'N', b'A', b'M', b'E', b'S', b'\0', b'\0'];

static PLAYPAL_LUMP_NAME: &'static [u8, ..8] =
    &[b'P', b'L', b'A', b'Y', b'P', b'A', b'L', b'\0'];

static COLORMAP_LUMP_NAME: &'static [u8, ..8] =
    &[b'C', b'O', b'L', b'O', b'R', b'M', b'A', b'P'];

static F_START_LUMP_NAME: &'static [u8, ..8] =
    &[b'F', b'_', b'S', b'T', b'A', b'R', b'T', b'\0'];

static F_END_LUMP_NAME: &'static [u8, ..8] =
    &[b'F', b'_', b'E', b'N', b'D', b'\0', b'\0', b'\0'];

static TEXTURE_LUMP_NAMES: &'static [[u8, ..8]] =
    &[[b'T', b'E', b'X', b'T', b'U', b'R', b'E', b'1'],
      [b'T', b'E', b'X', b'T', b'U', b'R', b'E', b'2']];


fn read_patches(wad: &mut Archive)
        -> Result<Vec<(WadName, Option<Image>)>, String> {
    let pnames_buffer = wad.read_lump_by_name(PNAMES_LUMP_NAME);
    let mut lump = BufReader::new(pnames_buffer.as_slice());

    let num_patches = io_try!(lump.read_le_u32()) as uint;
    let mut patches = Vec::with_capacity(num_patches);

    patches.reserve_additional(num_patches);
    let mut missing_patches = 0u;
    info!("Reading {} patches....", num_patches);
    for _ in range(0, num_patches) {
        let name = read_binary::<WadName, _>(&mut lump);
        let patch = wad.get_lump_index(&name).map(|index| {
            let patch_buffer = wad.read_lump(index);
            Image::from_buffer(patch_buffer.as_slice())
        });
        if patch.is_none() { missing_patches += 1; }
        patches.push((name, patch));
    }
    warn!("{} missing patches.", missing_patches);
    Ok(patches)
}


fn read_textures(lump_buffer: &[u8], patches: &[(WadName, Option<Image>)],
                 textures: &mut HashMap<Vec<u8>, Image>)
        -> Result<uint, String> {
    let mut lump = BufReader::new(lump_buffer.as_slice());
    let num_textures = io_try!(lump.read_le_u32()) as uint;
    let current_num_textures = textures.len();
    textures.reserve(current_num_textures + num_textures);

    let mut offsets = BufReader::new({
        let begin = io_try!(lump.tell()) as uint;
        let size = num_textures * mem::size_of::<u32>();
        lump_buffer.slice(begin, begin + size)
    });

    for _ in range(0, num_textures) {
        io_try!(lump.seek(io_try!(offsets.read_le_u32()) as i64, SeekSet));
        let header = read_binary::<WadTextureHeader, _>(&mut lump);
        let mut image = Image::new_from_header(&header);

        for i_patch in range(0, header.num_patches) {
            let pref = read_binary::<WadTexturePatchRef, _>(&mut lump);
            let (off_x, off_y) = (pref.origin_x as int, pref.origin_y as int);
            match patches[pref.patch as uint] {
                (_, Some(ref patch)) => {
                    image.blit(patch,
                               off_x, if off_y < 0 { 0 } else { off_y },
                               i_patch == 0);
                },
                (ref patch_name, None) => {
                    return Err(format!("Texture {} uses missing patch {}.",
                               str::from_utf8(header.name),
                               str::from_utf8(patch_name)));
                }
            }
        }

        textures.insert(name_toupper(header.name), image);
    }
    Ok(num_textures)
}

fn read_flats(wad: &mut Archive) -> Result<HashMap<Vec<u8>, Flat>, String> {
    let start = match wad.get_lump_index(F_START_LUMP_NAME) {
        Some(index) => index + 1,
        None => return Err(String::from_str("Missing F_START."))
    };

    let end = match wad.get_lump_index(F_END_LUMP_NAME) {
        Some(index) => index,
        None => return Err(String::from_str("Missing F_END."))
    };

    let mut flats = HashMap::with_capacity(end - start);
    for i_lump in range(start, end) {
        if wad.is_virtual_lump(i_lump) { continue; }
        let lump = wad.read_lump(i_lump);
        flats.insert(name_toupper(wad.get_lump_name(i_lump)), lump);
    }

    Ok(flats)
}

