use archive::{Archive, InArchive};
use error::ErrorKind::MissingRequiredPatch;
use error::Result;
use gfx::Texture;
use gl;
use image::Image;
use math::{Vec2, Vec2f};
use name::WadName;
use num::Float;
use read::WadRead;
use std::cmp;
use std::collections::BTreeMap;
use std::mem;
use time;
use types::{WadTextureHeader, WadTexturePatchRef};

pub type Palette = [u8; 256 * 3];
pub type Colormap = [u8; 256];
pub type Flat = Vec<u8>;


#[derive(Copy, Clone, Debug)]
pub struct Bounds {
    pub pos: Vec2f,
    pub size: Vec2f,
    pub num_frames: usize,
    pub row_height: usize,
}

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

fn search_for_frame<'a>(search_for: &WadName, animations: &'a Vec<Vec<WadName>>)
        -> Option<&'a [WadName]> {
    for animation in animations.iter() {
        for frame in animation.iter() {
            if search_for == frame { return Some(&animation); }
        }
    }
    None
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
        for lump_name in TEXTURE_LUMP_NAMES.iter() {
            let lump_index = match wad.named_lump_index(lump_name) {
                Some(i) => i,
                None => {
                    info!("     0 textures in {}", String::from_utf8_lossy(lump_name));
                    continue
                }
            };
            let num_textures = try!(read_textures(
                    &try!(wad.read_lump(lump_index)), &patches, &mut textures).in_archive(wad));
            info!("  {:4} textures in {}", num_textures, String::from_utf8_lossy(lump_name));
        }
        info!("Done in {:.4}s.", time::precise_time_s() - t0);

        // Read flats.
        let flats = try!(read_flats(wad));
        info!("  {:4} flats", flats.len());

        // Read sprites
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

    pub fn num_patches(&self) -> usize { self.patches.len() }
    pub fn patch(&self, index: usize) -> Option<&Image> {
        self.patches[index].1.as_ref()
    }

    pub fn num_palettes(&self) -> usize { self.palettes.len() }
    pub fn palette(&self, index: usize) -> &Palette {
        &self.palettes[index]
    }

    pub fn num_colormaps(&self) -> usize { self.colormaps.len() }
    pub fn colormap(&self, index: usize) -> &Colormap {
        &self.colormaps[index]
    }

    pub fn build_palette_texture(&self,
                                 palette: usize,
                                 colormap_start: usize,
                                 colormap_end: usize) -> Texture {
        let num_colormaps = colormap_end - colormap_start;
        let mut data = vec![0u8; 256 * num_colormaps * 3];
        let palette = &self.palettes[palette];
        for i_colormap in colormap_start .. colormap_end {
            for i_color in 0 .. 256 {
                let rgb = &palette[self.colormaps[i_colormap][i_color] as usize * 3..][..3];
                data[0 + i_color * 3 + i_colormap * 256 * 3] = rgb[0];
                data[1 + i_color * 3 + i_colormap * 256 * 3] = rgb[1];
                data[2 + i_color * 3 + i_colormap * 256 * 3] = rgb[2];
            }
        }

        let mut palette_tex = Texture::new(gl::TEXTURE_2D);
        palette_tex.bind(gl::TEXTURE0);
        palette_tex
            .set_filters_nearest()
            .data_rgb_u8(0, 256, num_colormaps, &data)
            .unbind(gl::TEXTURE0);
        palette_tex
    }

    pub fn build_colormap_texture(&self) -> Texture {
        let mut colormap_tex = Texture::new(gl::TEXTURE_2D);
        colormap_tex.bind(gl::TEXTURE0);
        colormap_tex
            .set_filters_nearest()
            .data_red_u8(0, 256, self.colormaps.len(), &self.colormaps)
            .unbind(gl::TEXTURE0);
        colormap_tex
    }


    pub fn build_texture_atlas<'a, T: Iterator<Item = &'a WadName>>(
            &'a self, names_iter: T) -> (Texture, BoundsLookup) {
        let images = ordered_atlas_entries(
            &self.animated_walls,
            |n| self.texture(n),
            names_iter);
        if images.len() == 0 {
            return (Texture::new(gl::TEXTURE_2D), BoundsLookup::new());
        }

        let num_pixels = images
            .iter().map(|t| t.1.num_pixels()).fold(0, |x, y| x + y);
        let min_atlas_width = cmp::min(
            128, next_pow2(images.iter().map(|t| t.1.width()).max().unwrap()));
        let min_atlas_height = 128;
        let max_size = 4096;

        let next_size = |w: &mut usize, h: &mut usize| {
            loop {
                if *w <= *h {
                    if *w == max_size { panic!("Could not fit wall atlas."); }
                    *w *= 2; *h = min_atlas_height;
                } else { *h *= 2; }

                if *w * *h >= num_pixels { break; }
            }
        };

        fn img_bound((x_offset, y_offset, row_height): (isize, isize, usize),
                     img: &Image,
                     num_frames: usize)
                    -> Bounds {
            Bounds {
                pos: Vec2::new(x_offset as f32, y_offset as f32),
                size: Vec2::new(img.width() as f32, img.height() as f32),
                num_frames: num_frames,
                row_height: row_height
            }
        }


        let (mut atlas_width, mut atlas_height) = (min_atlas_width,
                                                   min_atlas_height);
        next_size(&mut atlas_width, &mut atlas_height);

        let mut transposed = false;
        let mut offsets = Vec::with_capacity(images.len());
        loop {
            let mut x_offset = 0;
            let mut y_offset = 0;
            let mut failed = false;
            let mut max_height = 0;
            for &(_, image, _, _) in images.iter() {
                let (width, height) = (image.width(), image.height());
                if x_offset + width > atlas_width {
                    x_offset = 0;
                    y_offset += max_height;
                    max_height = 0;
                }
                if height > max_height { max_height = height; }
                if y_offset + height > atlas_height {
                    failed = true;
                    break;
                }
                offsets.push((x_offset as isize, y_offset as isize, max_height as usize));
                x_offset += width;
            }

            if failed {
                offsets.clear();

                // Try swapping width and height to see if it fits that way.
                mem::swap(&mut atlas_width, &mut atlas_height);
                transposed = !transposed;
                if transposed && atlas_width != atlas_height {
                    continue;
                }

                // If all else fails try a larger size for the atlas.
                transposed = false;
                next_size(&mut atlas_width, &mut atlas_height);
            } else {
                break;
            }
        }
        let (atlas_width, atlas_height) = (atlas_width, atlas_height);

        assert!(offsets.len() == images.len());
        let mut atlas = Image::new(atlas_width, atlas_height);
        let mut bound_map = BTreeMap::new();
        for (i, (name, image, frame_offset, num_frames)) in images.into_iter().enumerate() {
            atlas.blit(image, offsets[i].0, offsets[i].1, true);
            bound_map.insert(*name,
                             img_bound(offsets[i - frame_offset],
                                       image,
                                       num_frames));
        }

        let mut tex = Texture::new(gl::TEXTURE_2D);
        tex.bind(gl::TEXTURE0);
        tex.set_filters_nearest()
           .data_rg_u8(0, atlas_width, atlas_height, atlas.pixels())
           .unbind(gl::TEXTURE0);

        info!("Wall texture atlas size: {}x{}", atlas_width, atlas_height);
        (tex, bound_map)
    }

    pub fn build_flat_atlas<'a, T: Iterator<Item = &'a WadName>>(
            &'a self, names_iter: T) -> (Texture, BoundsLookup) {
        let names = ordered_atlas_entries(
            &self.animated_flats, |n| self.flat(n),
            names_iter);
        let num_names = names.len();

        let width = next_pow2((num_names as f64).sqrt().ceil() as usize * 64);
        let flats_per_row = width / 64;

        let num_rows = (num_names as f64 / flats_per_row as f64).ceil() as usize;
        let height = next_pow2(num_rows * 64);

        let mut offsets = BTreeMap::new();
        let mut data = vec![255u8; width * height];
        let (mut row, mut column) = (0, 0);
        info!("Flat atlas size: {}x{} ({}, {})", width, height, flats_per_row,
                                                 num_rows);
        let mut anim_start_pos = Vec2::zero();
        for (name, flat, frame_offset, num_frames) in names.into_iter() {
            let x_offset = column * 64;
            let y_offset = row * 64;

            if frame_offset == 0 {
               anim_start_pos = Vec2::new(x_offset as f32, y_offset as f32);
            }
            offsets.insert(*name, Bounds {
                pos: anim_start_pos,
                size: Vec2::new(64.0, 64.0),
                num_frames: num_frames,
                row_height: 64,
            });

            for y in 0 .. 64 {
                for x in 0 .. 64 {
                    data[x_offset + x + (y + y_offset) * width]
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
           .data_red_u8(0, width, height, &data)
           .unbind(gl::TEXTURE0);

        (tex, offsets)
    }
}


fn next_pow2(x: usize) -> usize {
    let mut pow2 = 1;
    while pow2 < x { pow2 *= 2; }
    pow2
}


const TEXTURE_LUMP_NAMES: &'static [[u8; 8]] =
    &[[b'T', b'E', b'X', b'T', b'U', b'R', b'E', b'1'],
      [b'T', b'E', b'X', b'T', b'U', b'R', b'E', b'2']];


fn read_patches(wad: &Archive) -> Result<Vec<(WadName, Option<Image>)>> {
    let pnames_buffer = try!(wad.read_required_named_lump(b"PNAMES\0\0"));
    let mut lump = &pnames_buffer[..];

    let num_patches = try!(lump.wad_read::<u32>()) as usize;
    let mut patches = Vec::with_capacity(num_patches);

    patches.reserve(num_patches);
    let mut missing_patches = 0usize;
    info!("Reading {} patches....", num_patches);
    let t0 = time::precise_time_s();
    for _ in 0 .. num_patches {
        let name = try!(lump.wad_read::<WadName>());
        match wad.named_lump_index(&name) {
            Some(index) => {
                patches.push((name, Some(Image::from_buffer(&try!(wad.read_lump(index))))));
            }
            None => {
                missing_patches += 1;
                patches.push((name, None));
            },
        }
    }
    let time = time::precise_time_s() - t0;
    info!("Done in {:.4}s; {} missing patches.", time, missing_patches);
    Ok(patches)
}


fn read_sprites(wad: &Archive, textures: &mut BTreeMap<WadName, Image>) -> Result<usize> {
    let start_index =
        try!(wad.required_named_lump_index(b"S_START\0")) + 1;
    let end_index = try!(wad.required_named_lump_index(b"S_END\0\0\0"));
    info!("Reading {} sprites....", end_index - start_index);
    let t0 = time::precise_time_s();
    for index in start_index .. end_index {
        textures.insert(*wad.lump_name(index), Image::from_buffer(&try!(wad.read_lump(index))));
    }
    let time = time::precise_time_s() - t0;
    info!("Done in {:.4}s.", time);
    Ok(end_index - start_index)
}

fn read_textures(lump_buffer: &[u8], patches: &[(WadName, Option<Image>)],
                 textures: &mut BTreeMap<WadName, Image>)
        -> Result<usize> {
    let mut lump = lump_buffer;
    let num_textures = try!(lump.wad_read::<u32>()) as usize;

    let mut offsets = &lump[..num_textures * mem::size_of::<u32>()];

    for _ in 0 .. num_textures {
        lump = &lump_buffer[try!(offsets.wad_read::<u32>()) as usize..];
        let header = try!(lump.wad_read::<WadTextureHeader>());
        let mut image = Image::new_from_header(&header);

        for i_patch in 0 .. header.num_patches {
            let pref = try!(lump.wad_read::<WadTexturePatchRef>());
            let (off_x, off_y) =
                    (pref.origin_x as isize, pref.origin_y as isize);
            match patches[pref.patch as usize] {
                (_, Some(ref patch)) => {
                    image.blit(patch,
                               off_x, if off_y < 0 { 0 } else { off_y },
                               i_patch == 0);
                },
                (ref patch_name, None) => {
                    return Err(MissingRequiredPatch(header.name, *patch_name).into())
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
    for i_lump in start .. end {
        if wad.is_virtual_lump(i_lump) {
            continue;
        }
        let lump = try!(wad.read_lump(i_lump));
        flats.insert(*wad.lump_name(i_lump), lump);
    }

    Ok(flats)
}

pub fn ordered_atlas_entries<'b, 'a: 'b,
                             NameIteratorT: Iterator<Item = &'a WadName>,
                             ImageT,
                             ImageLookupT: Fn(&WadName) -> Option<&'b ImageT>>(
            animations: &'b Vec<Vec<WadName>>,
            image_lookup: ImageLookupT,
            names_iter: NameIteratorT)
        -> Vec<(&'b WadName, &'b ImageT, usize, usize)> {
    let mut frames_by_first_frame = BTreeMap::new();
    for name in names_iter {
        let maybe_frames = search_for_frame(name, animations);
        let first_frame = maybe_frames.map(|f| &f[0]).unwrap_or(name);
        frames_by_first_frame.insert(first_frame, maybe_frames);
    }
    let mut names = Vec::new();
    for (name, maybe_frames) in frames_by_first_frame.into_iter() {
        match maybe_frames {
            Some(frames) =>
                for (offset, frame) in frames.iter().enumerate() {
                    if let Some(image) = image_lookup(frame) {
                        names.push(
                            (frame, image, offset, frames.len()));
                    } else {
                        warn!("Unable to find texture/sprite: {}", frame);
                    }
                },
            None => {
                if let Some(image) = image_lookup(name) {
                    names.push((name, image, 0, 1))
                }
            },
        }
    }
    names
}

