use std::collections::HashMap;
use std::{mem, iter};
use std::io::{File, SeekSet};
use std::slice::raw;
use std::vec::Vec;
use std::str;
use std::ascii::ASCII_UPPER_MAP;

use super::types::{WadLump, WadInfo};
use super::util::{wad_type_from_info, read_binary, name_eq, name_toupper};

pub struct Archive {
    file: File,
    index_map: HashMap<Vec<u8>, uint>,
    lumps: Vec<LumpInfo>,
    levels: Vec<uint>,
}


impl Archive {
    pub fn open(path : &Path) -> Result<Archive, String> {
        let path_str = path.display();
        info!("Loading wad file '{}'...", path_str);


        // Open file, read and check header.
        let mut file = try!(File::open(path).map_err(|err| {
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
        let mut lumps = Vec::with_capacity(header.num_lumps as uint);
        let mut levels = Vec::with_capacity(32);
        let mut index_map = HashMap::new();

        file.seek(header.info_table_offset as i64, SeekSet).unwrap();
        for i_lump in iter::range(0, header.num_lumps) {
            let mut fileinfo = read_binary::<WadLump, _>(&mut file);
            for byte in fileinfo.name.iter_mut() {
                (*byte) = ASCII_UPPER_MAP[*byte as uint];
            }
            let fileinfo = fileinfo;
            index_map.insert(fileinfo.name.to_vec(), lumps.len());
            lumps.push(LumpInfo { name: fileinfo.name,
                                  offset: fileinfo.file_pos as i64,
                                  size: fileinfo.size as uint });

            if name_eq(&fileinfo.name, b"THINGS\0\0") {
                assert!(i_lump > 0);
                levels.push((i_lump - 1) as uint);
            }
        }

        Ok(Archive {
            file: file,
            lumps: lumps,
            index_map: index_map,
            levels: levels })
    }

    pub fn num_levels(&self) -> uint { self.levels.len() }

    pub fn get_level_lump_index(&self, level_index: uint) -> uint {
        self.levels[level_index]
    }

    pub fn get_level_name<'a>(&'a self, level_index: uint) -> &'a [u8, ..8] {
        self.get_lump_name(self.levels[level_index])
    }

    pub fn num_lumps(&self) -> uint { self.lumps.len() }

    pub fn get_lump_index(&self, name: &[u8, ..8]) -> Option<uint> {
        self.index_map.find(&name_toupper(name)).map(|x| *x)
    }

    pub fn get_lump_name<'a>(&'a self, lump_index: uint) -> &'a [u8, ..8] {
        &self.lumps[lump_index].name
    }

    pub fn is_virtual_lump(&self, lump_index: uint) -> bool {
        self.lumps[lump_index].size == 0
    }

    pub fn read_lump_by_name<T: Copy>(&mut self, name: &[u8, ..8]) -> Vec<T> {
        let index = self.get_lump_index(name).unwrap_or_else(
            || fail!("No such lump '{}'", str::from_utf8(name).unwrap()));
        self.read_lump(index)
    }

    pub fn read_lump<T: Copy>(&mut self, index: uint) -> Vec<T> {
        let info = self.lumps[index];
        assert!(info.size > 0);
        assert!(info.size % mem::size_of::<T>() == 0);
        let num_elems = info.size / mem::size_of::<T>();
        let mut buf = Vec::with_capacity(num_elems);
        self.file.seek(info.offset, SeekSet).unwrap();
        unsafe {
            buf.set_len(num_elems);
            raw::mut_buf_as_slice(
                buf.as_mut_ptr() as *mut u8, info.size,
                |slice| self.file.read_at_least(info.size, slice).unwrap());
        }
        buf
    }

    pub fn read_lump_single<T: Copy>(&mut self, index: uint) -> T {
        let info = self.lumps[index];
        assert!(info.size == mem::size_of::<T>());
        self.file.seek(info.offset, SeekSet).unwrap();
        read_binary(&mut self.file)
    }
}


struct LumpInfo {
    name   : [u8, ..8],
    offset : i64,
    size   : uint,
}
