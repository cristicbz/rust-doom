use std::collections::HashMap;
use std::{mem, iter};
use std::io::{File, SeekSet};
use std::slice::raw;
use std::vec::Vec;

static IWAD_HEADER: &'static [u8] = b"IWAD";
static PWAD_HEADER: &'static [u8] = b"PWAD";


struct LumpInfo {
    name   : [u8, ..8],
    offset : i64,
    size   : uint,
}

pub struct WadFile {
    file: File,
    index_map: HashMap<Vec<u8>, uint>,
    lumps: Vec<LumpInfo>,
}

impl WadFile {
    pub fn open(path : &Path) -> Result<WadFile, String> {
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
        let mut index_map = HashMap::new();

        file.seek(header.info_table_offset as i64, SeekSet).unwrap();
        for i_lump in iter::range(0, header.num_lumps) {
            let fileinfo = read_binary::<FileLump, _>(&mut file);
            index_map.insert(Vec::from_slice(&fileinfo.name), lumps.len());
            lumps.push(LumpInfo { name: fileinfo.name,
                                  offset: fileinfo.file_pos as i64,
                                  size: fileinfo.size as uint });
        }

        Ok(WadFile { file: file, lumps: lumps, index_map: index_map })
    }

    pub fn num_lumps(&self) -> uint { self.lumps.len() }

    pub fn lump_index_by_name(&self, name: &[u8, ..8]) -> Option<uint> {
        self.index_map.find(&Vec::from_slice(name)).map(|x| *x)
    }

    pub fn lump_by_name(&mut self, name: &[u8, ..8]) -> Vec<u8> {
        let index = self.lump_index_by_name(name).unwrap();
        self.lump_at(index)
    }

    pub fn lump_name_at<'a>(&'a self, index: uint) -> &'a [u8, ..8] {
        &self.lumps[index].name
    }

    pub fn lump_at<T: Copy>(&mut self, index: uint) -> Vec<T> {
        let info = self.lumps[index];
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
}

#[repr(C)]
#[repr(packed)]
struct WadInfo {
    identifier        : [u8, ..4],  // IWAD/PWAD
    num_lumps         : i32,
    info_table_offset : i32,
}

#[repr(C)]
#[repr(packed)]
struct FileLump {
    file_pos : i32,
    size     : i32,
    name     : [u8, ..8],
}

fn read_binary<T : Copy, R : Reader>(reader : &mut R) -> T {
    let mut loaded : T = unsafe { mem::zeroed() };
    let size = mem::size_of::<T>();
    unsafe {
        raw::mut_buf_as_slice(
            &mut loaded as *mut T as *mut u8, size,
            |buf| { reader.read_at_least(size, buf).unwrap() });
    };
    loaded
}

//fn read_binary_vec<T : Copy,
//                   R : Reader>(reader : &mut R, count : uint) -> Vec<T> {
//    let mut loaded = Vec::<T>::with_capacity(count);
//    unsafe {
//        loaded.set_len(count);
//        let num_bytes = mem::size_of::<T>() * count;
//        raw::mut_buf_as_slice(
//            loaded.as_mut_ptr() as *mut u8,
//            num_bytes,
//            |buf| { reader.read_at_least(num_bytes, buf) });
//    }
//    loaded
//}

enum WadType { Initial, Patch }

fn wad_type_from_info(wad_info : &WadInfo) -> Option<WadType> {
    let id : &[u8] = &wad_info.identifier;
    match id {
        IWAD_HEADER => Some(Initial),
        PWAD_HEADER => Some(Patch),
        _           => None
    }
}

