use std::collections::HashMap;
use std::io::{BufReader, Reader, SeekSet};
use std::{str, mem};

use super::Archive;
use super::image::Image;
use super::types::*;
use super::util::read_binary;


pub type Palette = [[u8, ..3], ..256];
pub type Colormap = [u8, ..256];


pub struct TextureDirectory {
    textures: HashMap<Vec<u8>, Image>,
    patches: Vec<(WadName, Option<Image>)>,
    palettes: Vec<Palette>,
    colormaps: Vec<Colormap>,
}


macro_rules! io_try(
    ($e:expr) => (try!($e.map_err(|e| String::from_str(e.desc))))
)

impl TextureDirectory {
    pub fn from_archive(wad: &mut Archive) -> Result<TextureDirectory, String> {
        info!("Reading texture directory...");
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
                          str::from_utf8(lump_name).unwrap());
                    continue
                }
            };
            let num_textures = try!(
                read_textures(wad.read_lump(lump_index).as_slice(),
                              patches.as_slice(), &mut textures));
            info!("  {:4} textures in {}",
                  num_textures, str::from_utf8(lump_name).unwrap());
        }
        let textures = textures;

        let palettes = try!(read_palettes(wad));
        let colormaps = try!(read_colormaps(wad));
        info!("  {:4} palettes", palettes.len());
        info!("  {:4} colormaps", colormaps.len());

        Ok(TextureDirectory {
            patches: patches,
            textures: textures,
            palettes: palettes,
            colormaps: colormaps
        })
    }

    pub fn get_texture<'a>(&'a self, name: &[u8]) -> Option<&'a Image> {
        self.textures.find(&Vec::from_slice(name))
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
}


static PNAMES_LUMP_NAME: &'static [u8, ..8] =
    &[b'P', b'N', b'A', b'M', b'E', b'S', b'\0', b'\0'];

static PLAYPAL_LUMP_NAME: &'static [u8, ..8] =
    &[b'P', b'L', b'A', b'Y', b'P', b'A', b'L', b'\0'];

static COLORMAP_LUMP_NAME: &'static [u8, ..8] =
    &[b'C', b'O', b'L', b'O', b'R', b'M', b'A', b'P'];

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
    for i_patch in range(0, num_patches) {
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

    for i_texture in range(0, num_textures) {
        io_try!(lump.seek(io_try!(offsets.read_le_u32()) as i64, SeekSet));
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
                    return Err(format!("Texture {} uses missing patch {}.",
                               str::from_utf8(header.name).unwrap(),
                               str::from_utf8(patch_name).unwrap()));
                }
            }
        }

        textures.insert(Vec::from_slice(header.name), image);
    }
    Ok(num_textures)
}


fn read_palettes(wad: &mut Archive) -> Result<Vec<Palette>, String> {
    Ok(wad.read_lump_by_name(PLAYPAL_LUMP_NAME))
}


fn read_colormaps(wad: &mut Archive) -> Result<Vec<Colormap>, String> {
    Ok(wad.read_lump_by_name(COLORMAP_LUMP_NAME))
}
