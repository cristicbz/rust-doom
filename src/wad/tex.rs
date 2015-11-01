use archive::{Archive, InArchive};
use error::ErrorKind::BadImage;
use error::Result;
use gfx::Bounds;
use image::Image;
use math::{Vec2, Vec2f};
use name::WadName;
use num::{Float, Zero};
use read::WadRead;
use std::cmp;
use std::collections::BTreeMap;
use std::mem;
use time;
use types::{WadTextureHeader, WadTexturePatchRef};

pub type Palette = [u8; 256 * 3];
pub type Colormap = [u8; 256];
pub type Flat = Vec<u8>;
pub type BoundsLookup = BTreeMap<WadName, Bounds>;

pub struct TextureDirectory {
    textures: BTreeMap<WadName, Image>,
    patches: Vec<(WadName, Option<Image>)>,
    palettes: Vec<Palette>,
    colormaps: Vec<Colormap>,
    flats: BTreeMap<WadName, Flat>,
    animated_walls: Vec<Vec<WadName>>,
    animated_flats: Vec<Vec<WadName>>,
}

pub struct MappedPalette {
    pub pixels: Vec<u8>,
    pub colormaps: usize,
}

pub struct TransparentImage {
    pub pixels: Vec<u16>,
    pub size: Vec2<usize>,
}

pub struct OpaqueImage {
    pub pixels: Vec<u8>,
    pub size: Vec2<usize>,
}


impl TextureDirectory {
    pub fn from_archive(wad: &Archive) -> Result<TextureDirectory> {
        info!("Reading texture directory...");

        // Read palettes & colormaps.
        let palettes = try!(wad.read_required_named_lump(b"PLAYPAL\0"));
        let colormaps = try!(wad.read_required_named_lump(b"COLORMAP"));
        info!("  {:4} palettes", palettes.len());
        info!("  {:4} colormaps", colormaps.len());

        // Read patches.
        let patches = try!(read_patches(wad).in_archive(wad));
        info!("  {:4} patches", patches.len());

        // Read textures.
        let t0 = time::precise_time_s();
        info!("Reading & assembling textures...");
        let mut textures = BTreeMap::new();
        for &lump_name in TEXTURE_LUMP_NAMES {
            let lump_index = match wad.named_lump_index(lump_name) {
                Some(i) => i,
                None => {
                    info!("     0 textures in {}", String::from_utf8_lossy(lump_name));
                    continue;
                }
            };
            let num_textures = try!(read_textures(&try!(wad.read_lump(lump_index)),
                                                  &patches,
                                                  &mut textures)
                                        .in_archive(wad));
            info!("  {:4} textures in {}",
                  num_textures,
                  String::from_utf8_lossy(lump_name));
        }
        info!("Done in {:.4}s.", time::precise_time_s() - t0);

        // Read flats.
        let flats = try!(read_flats(wad));
        info!("  {:4} flats", flats.len());

        // Read sprites.
        let num_sprites = try!(read_sprites(wad, &mut textures));
        info!("  {:4} sprites", num_sprites);

        Ok(TextureDirectory {
            patches: patches,
            textures: textures,
            palettes: palettes,
            colormaps: colormaps,
            flats: flats,
            animated_walls: wad.metadata().animations.walls.clone(),
            animated_flats: wad.metadata().animations.flats.clone(),
        })
    }

    pub fn texture(&self, name: &WadName) -> Option<&Image> {
        self.textures.get(name)
    }
    pub fn flat(&self, name: &WadName) -> Option<&Flat> {
        self.flats.get(name)
    }

    pub fn num_patches(&self) -> usize {
        self.patches.len()
    }
    pub fn patch(&self, index: usize) -> Option<&Image> {
        self.patches[index].1.as_ref()
    }

    pub fn num_palettes(&self) -> usize {
        self.palettes.len()
    }
    pub fn palette(&self, index: usize) -> &Palette {
        &self.palettes[index]
    }

    pub fn num_colormaps(&self) -> usize {
        self.colormaps.len()
    }
    pub fn colormap(&self, index: usize) -> &Colormap {
        &self.colormaps[index]
    }

    pub fn build_palette_texture(&self,
                                 palette: usize,
                                 colormap_start: usize,
                                 colormap_end: usize)
                                 -> MappedPalette {
        let num_colormaps = colormap_end - colormap_start;
        let mut data = vec![0u8; 256 * num_colormaps * 3];
        let palette = &self.palettes[palette];
        for i_colormap in colormap_start..colormap_end {
            for i_color in 0..256 {
                let rgb = &palette[self.colormaps[i_colormap][i_color] as usize * 3..][..3];
                data[i_color * 3 + i_colormap * 256 * 3] = rgb[0];
                data[1 + i_color * 3 + i_colormap * 256 * 3] = rgb[1];
                data[2 + i_color * 3 + i_colormap * 256 * 3] = rgb[2];
            }
        }

        MappedPalette {
            pixels: data,
            colormaps: colormap_end - colormap_start + 1,
        }
    }


    pub fn build_texture_atlas<'a, T>(&'a self, names_iter: T) -> (TransparentImage, BoundsLookup)
        where T: IntoIterator<Item = &'a WadName>
    {
        let entries = ordered_atlas_entries(&self.animated_walls, |n| self.texture(&n), names_iter);
        let max_image_width = if let Some(width) = entries.iter().map(|e| e.image.width()).max() {
            width
        } else {
            let image = TransparentImage {
                pixels: Vec::new(),
                size: Vec2::zero(),
            };
            return (image, BoundsLookup::new());
        };
        let num_pixels = entries.iter().map(|e| e.image.num_pixels()).fold(0, |x, y| x + y);
        let min_atlas_size = Vec2::new(cmp::min(128, next_pow2(max_image_width)), 128);
        let max_size = 4096;

        let next_size = |size: &mut Vec2<usize>| {
            loop {
                if size[0] <= size[1] {
                    if size[0] == max_size {
                        panic!("Could not fit wall atlas.");
                    }
                    size[0] *= 2;
                    size[1] = 128;
                } else {
                    size[1] *= 2;
                }

                if size[0] * size[1] >= num_pixels {
                    break;
                }
            }
        };

        let mut atlas_size = min_atlas_size;
        next_size(&mut atlas_size);

        let mut transposed = false;
        let mut positions = Vec::with_capacity(entries.len());
        loop {
            let mut offset = Vec2::zero();
            let mut failed = false;
            let mut row_height = 0;
            for &AtlasEntry { image, .. } in &entries {
                let size = image.size();
                if offset[0] + size[0] > atlas_size[0] {
                    offset[0] = 0;
                    offset[1] += row_height;
                    row_height = 0;
                }
                if size[1] > row_height {
                    row_height = size[1];
                }
                if offset[1] + size[1] > atlas_size[1] {
                    failed = true;
                    break;
                }
                positions.push(AtlasPosition {
                    offset: Vec2::new(offset[0] as isize, offset[1] as isize),
                    row_height: row_height,
                });
                offset[0] += size[0];
            }

            if failed {
                positions.clear();

                // Try swapping width and height to see if it fits that way.
                atlas_size.swap();
                transposed = !transposed;
                if transposed && atlas_size[0] != atlas_size[1] {
                    continue;
                }

                // If all else fails try a larger size for the atlas.
                transposed = false;
                next_size(&mut atlas_size);
            } else {
                break;
            }
        }
        let atlas_size = atlas_size;

        assert!(positions.len() == entries.len());
        // TODO(cristicbz): This should probably split things into multiple atlases or
        // something, but realistically, I'm never going to implement that.
        let mut atlas = Image::new(atlas_size[0], atlas_size[1]).ok().expect("atlas too big");
        let mut bound_map = BTreeMap::new();
        for (i, entry) in entries.iter().enumerate() {
            atlas.blit(entry.image, positions[i].offset, true);
            bound_map.insert(entry.name,
                             img_bound(&positions[i - entry.frame_offset], entry));
        }

        let tex = TransparentImage {
            size: atlas_size,
            pixels: atlas.into_pixels(),
        };

        info!("Wall texture atlas size: {:?}", atlas_size);
        (tex, bound_map)
    }

    pub fn build_flat_atlas<'a, T>(&'a self, names_iter: T) -> (OpaqueImage, BoundsLookup)
        where T: IntoIterator<Item = &'a WadName>
    {
        let names = ordered_atlas_entries(&self.animated_flats, |n| self.flat(&n), names_iter);
        let num_names = names.len();

        let width = next_pow2((num_names as f64).sqrt().ceil() as usize * 64);
        let flats_per_row = width / 64;

        let num_rows = (num_names as f64 / flats_per_row as f64).ceil() as usize;
        let height = next_pow2(num_rows * 64);

        let mut offsets = BTreeMap::new();
        let mut data = vec![255u8; width * height];
        let (mut row, mut column) = (0, 0);
        info!("Flat atlas size: {}x{} ({}, {})",
              width,
              height,
              flats_per_row,
              num_rows);
        let mut anim_start_pos = Vec2::zero();
        for AtlasEntry { name, image, frame_offset, num_frames } in names.into_iter() {
            let offset = Vec2::new(column * 64, row * 64);
            if frame_offset == 0 {
                anim_start_pos = Vec2::new(offset[0] as f32, offset[1] as f32);
            }
            offsets.insert(name,
                           Bounds {
                               pos: anim_start_pos,
                               size: Vec2::new(64.0, 64.0),
                               num_frames: num_frames,
                               row_height: 64,
                           });

            for y in 0..64 {
                for x in 0..64 {
                    data[offset[0] + x + (y + offset[1]) * width] = image[x + y * 64];
                }
            }

            column += 1;
            if column == flats_per_row {
                column = 0;
                row += 1;
            }
        }

        let tex = OpaqueImage {
            pixels: data,
            size: Vec2::new(width, height),
        };
        (tex, offsets)
    }
}

struct AtlasEntry<'a, ImageType: 'a> {
    name: WadName,
    image: &'a ImageType,
    frame_offset: usize,
    num_frames: usize,
}

struct AtlasPosition {
    offset: Vec2<isize>,
    row_height: usize,
}

fn next_pow2(x: usize) -> usize {
    let mut pow2 = 1;
    while pow2 < x {
        pow2 *= 2;
    }
    pow2
}


const TEXTURE_LUMP_NAMES: &'static [&'static [u8; 8]] = &[b"TEXTURE1", b"TEXTURE2"];

fn read_patches(wad: &Archive) -> Result<Vec<(WadName, Option<Image>)>> {
    let pnames_buffer = try!(wad.read_required_named_lump(b"PNAMES\0\0"));
    let mut lump = &pnames_buffer[..];

    let num_patches = try!(lump.wad_read::<u32>()) as usize;
    let mut patches = Vec::with_capacity(num_patches);

    patches.reserve(num_patches);
    let mut missing_patches = 0usize;
    info!("Reading {} patches....", num_patches);
    let t0 = time::precise_time_s();
    for _ in 0..num_patches {
        let name = try!(lump.wad_read::<WadName>());
        match wad.named_lump_index(&name) {
            Some(index) => {
                let image = match Image::from_buffer(&try!(wad.read_lump(index))) {
                    Ok(i) => Some(i),
                    Err(e) => {
                        warn!("Skipping patch: {}", BadImage(name, e));
                        None
                    }
                };

                patches.push((name, image));
            }
            None => {
                missing_patches += 1;
                patches.push((name, None));
            }
        }
    }
    let time = time::precise_time_s() - t0;
    info!("Done in {:.4}s; {} missing patches.", time, missing_patches);
    Ok(patches)
}

fn img_bound(pos: &AtlasPosition, entry: &AtlasEntry<Image>) -> Bounds {
    Bounds {
        pos: Vec2f::new(pos.offset[0] as f32, pos.offset[1] as f32),
        size: Vec2f::new(entry.image.width() as f32, entry.image.height() as f32),
        num_frames: entry.num_frames,
        row_height: pos.row_height,
    }
}

fn ordered_atlas_entries<'a, 'b, N, I, L>(animations: &'b [Vec<WadName>],
                                          image_lookup: L,
                                          names_iter: N)
                                          -> Vec<AtlasEntry<I>>
    where N: IntoIterator<Item = &'a WadName>,
          L: Fn(WadName) -> Option<&'b I>,
          'a: 'b
{
    let mut frames_by_first_frame = BTreeMap::new();
    for name in names_iter {
        let maybe_frames = search_for_frame(name, animations);
        let first_frame = maybe_frames.map(|f| &f[0]).unwrap_or(name);
        frames_by_first_frame.insert(first_frame, maybe_frames);
    }
    let mut entries = Vec::with_capacity(frames_by_first_frame.len());
    for (&name, maybe_frames) in frames_by_first_frame.into_iter() {
        match maybe_frames {
            Some(frames) => for (offset, &frame) in frames.iter().enumerate() {
                if let Some(image) = image_lookup(frame) {
                    entries.push(AtlasEntry {
                        name: frame,
                        image: image,
                        frame_offset: offset,
                        num_frames: frames.len(),
                    });
                } else {
                    warn!("Unable to find texture/sprite: {}", frame);
                }
            },
            None => if let Some(image) = image_lookup(name) {
                entries.push(AtlasEntry {
                    name: name,
                    image: image,
                    frame_offset: 0,
                    num_frames: 1,
                });
            },
        }
    }
    entries
}

fn search_for_frame<'a>(search_for: &WadName,
                        animations: &'a [Vec<WadName>])
                        -> Option<&'a [WadName]> {
    animations.iter()
              .find(|animation| animation.iter().any(|frame| frame == search_for))
              .map(|animation| &animation[..])
}


fn read_sprites(wad: &Archive, textures: &mut BTreeMap<WadName, Image>) -> Result<usize> {
    let start_index = try!(wad.required_named_lump_index(b"S_START\0")) + 1;
    let end_index = try!(wad.required_named_lump_index(b"S_END\0\0\0"));
    info!("Reading {} sprites....", end_index - start_index);
    let t0 = time::precise_time_s();
    for index in start_index..end_index {
        let name = *wad.lump_name(index);
        match Image::from_buffer(&try!(wad.read_lump(index))) {
            Ok(texture) => {
                textures.insert(name, texture);
            }
            Err(e) => {
                warn!("Skipping sprite: {}", BadImage(name, e));
                continue;
            }
        }
    }
    let time = time::precise_time_s() - t0;
    info!("Done in {:.4}s.", time);
    Ok(end_index - start_index)
}

fn read_textures(lump_buffer: &[u8],
                 patches: &[(WadName, Option<Image>)],
                 textures: &mut BTreeMap<WadName, Image>)
                 -> Result<usize> {
    let mut lump = lump_buffer;
    let num_textures = try!(lump.wad_read::<u32>()) as usize;

    let mut offsets = &lump[..num_textures * mem::size_of::<u32>()];

    for _ in 0..num_textures {
        lump = &lump_buffer[try!(offsets.wad_read::<u32>()) as usize..];
        let header = try!(lump.wad_read::<WadTextureHeader>());
        let mut image = match Image::new_from_header(&header) {
            Ok(image) => image,
            Err(e) => {
                warn!("Skipping texture: {}", BadImage(header.name, e));
                continue;
            }
        };

        for i_patch in 0..header.num_patches {
            let pref = try!(lump.wad_read::<WadTexturePatchRef>());
            let offset = Vec2::new(pref.origin_x as isize,
                                   if pref.origin_y <= 0 {
                                       0
                                   } else {
                                       pref.origin_y as isize
                                   });
            match patches.get(pref.patch as usize) {
                Some(&(_, Some(ref patch))) => {
                    image.blit(patch, offset, i_patch == 0);
                }
                Some(&(ref patch_name, None)) => {
                    warn!("PatchRef {}, required by {} is missing.",
                          patch_name,
                          header.name);
                }
                None => {
                    warn!("PatchRef index {} out of bounds ({}) in {}, skipping.",
                          pref.patch,
                          patches.len(),
                          header.name);
                }
            }
        }

        textures.insert(header.name, image);
    }
    Ok(num_textures)
}

fn read_flats(wad: &Archive) -> Result<BTreeMap<WadName, Flat>> {
    let start = try!(wad.required_named_lump_index(b"F_START\0"));
    let end = try!(wad.required_named_lump_index(b"F_END\0\0\0"));
    let mut flats = BTreeMap::new();
    for i_lump in start..end {
        if wad.is_virtual_lump(i_lump) {
            continue;
        }
        let lump = try!(wad.read_lump(i_lump));
        flats.insert(*wad.lump_name(i_lump), lump);
    }

    Ok(flats)
}
