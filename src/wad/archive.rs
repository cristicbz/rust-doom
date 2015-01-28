use std::collections::HashMap;
use std::{mem, iter};
use std::old_io::{File, SeekSet};
use std::slice;
use std::vec::Vec;

use super::meta::WadMetadata;
use super::types::{WadLump, WadInfo, WadName, WadNameCast};
use super::util::{wad_type_from_info, read_binary};

pub struct Archive {
    file: File,
    index_map: HashMap<WadName, usize>,
    lumps: Vec<LumpInfo>,
    levels: Vec<usize>,
    meta: WadMetadata,
}


impl Archive {
    pub fn open(wad_path: &Path, meta_path: &Path) -> Result<Archive, String> {
        let path_str = wad_path.display();
        info!("Loading wad file '{}'...", path_str);


        // Open file, read and check header.
        let mut file = try!(File::open(wad_path).map_err(|err| {
            format!("Could not open WAD file '{}': {}", path_str, err)
        }));
        let header = read_binary::<WadInfo, _>(&mut file);
        match wad_type_from_info(&header) {
            None =>
                return Err(format!(
                    "Invalid WAD file '{}': Incorrect header id.", path_str)),
            _ => {}
        };

        // Read lump info.
        let mut lumps = Vec::with_capacity(header.num_lumps as usize);
        let mut levels = Vec::with_capacity(32);
        let mut index_map = HashMap::new();

        file.seek(header.info_table_offset as i64, SeekSet).unwrap();
        for i_lump in iter::range(0, header.num_lumps) {
            let mut fileinfo = read_binary::<WadLump, _>(&mut file);
            fileinfo.name.canonicalise();
            let fileinfo = fileinfo;
            index_map.insert(fileinfo.name, lumps.len());
            lumps.push(LumpInfo { name: fileinfo.name,
                                  offset: fileinfo.file_pos as i64,
                                  size: fileinfo.size as usize });

            if fileinfo.name == b"THINGS\0\0".to_wad_name() {
                assert!(i_lump > 0);
                levels.push((i_lump - 1) as usize);
            }
        }


        // Read metadata.
        let meta = try!(WadMetadata::from_file(meta_path));

        Ok(Archive {
            meta: meta,
            file: file,
            lumps: lumps,
            index_map: index_map,
            levels: levels })
    }

    pub fn num_levels(&self) -> usize { self.levels.len() }

    pub fn get_level_lump_index(&self, level_index: usize) -> usize {
        self.levels[level_index]
    }

    pub fn get_level_name(&self, level_index: usize) -> &WadName {
        self.get_lump_name(self.levels[level_index])
    }

    pub fn num_lumps(&self) -> usize { self.lumps.len() }

    pub fn get_lump_index(&self, name: &WadName) -> Option<usize> {
        self.index_map.get(name).map(|x| *x)
    }

    pub fn get_lump_name(&self, lump_index: usize) -> &WadName {
        &self.lumps[lump_index].name
    }

    pub fn is_virtual_lump(&self, lump_index: usize) -> bool {
        self.lumps[lump_index].size == 0
    }

    pub fn read_lump_by_name<T: Copy>(&mut self, name: &WadName) -> Vec<T> {
        let index = self.get_lump_index(name).unwrap_or_else(
            || panic!("No such lump '{}'", name));
        self.read_lump(index)
    }

    pub fn read_lump<T: Copy>(&mut self, index: usize) -> Vec<T> {
        let info = self.lumps[index];
        assert!(info.size > 0);
        assert!(info.size % mem::size_of::<T>() == 0);
        let num_elems = info.size / mem::size_of::<T>();
        let mut buf = Vec::with_capacity(num_elems);
        self.file.seek(info.offset, SeekSet).unwrap();
        unsafe {
            buf.set_len(num_elems);
            self.file.read_at_least(info.size,
                    slice::from_raw_mut_buf(
                        &(buf.as_mut_ptr() as *mut u8), info.size)).unwrap();
        }
        buf
    }

    pub fn read_lump_single<T: Copy>(&mut self, index: usize) -> T {
        let info = self.lumps[index];
        assert!(info.size == mem::size_of::<T>());
        self.file.seek(info.offset, SeekSet).unwrap();
        read_binary(&mut self.file)
    }

    pub fn get_metadata(&self) -> &WadMetadata { &self.meta }
}

#[derive(Copy)]
struct LumpInfo {
    name  : WadName,
    offset: i64,
    size  : usize,
}
