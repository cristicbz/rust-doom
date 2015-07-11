use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::File;
use std::io::{Seek, SeekFrom};
use std::mem;
use std::path::Path;
use std::path::PathBuf;
use std::slice;
use std::vec::Vec;
use std::result::Result as StdResult;

use common::ReadExt;
use meta::WadMetadata;
use types::{WadLump, WadInfo, WadName, WadNameCast};
use util::wad_type_from_info;
use error::{Result, Error, ErrorKind, InFile};
use error::ErrorKind::{BadWadHeader, MissingRequiredLump};

pub struct Archive {
    file: RefCell<File>,
    index_map: HashMap<WadName, usize>,
    lumps: Vec<LumpInfo>,
    levels: Vec<usize>,
    meta: WadMetadata,
    path: PathBuf,
}

impl Archive {
    pub fn open<W, M>(wad_path: &W, meta_path: &M) -> Result<Archive>
            where W: AsRef<Path> + Debug,
                  M: AsRef<Path> + Debug {
        let wad_path = wad_path.as_ref().to_owned();
        info!("Loading wad file '{:?}'...", wad_path);

        // Open file, read and check header.
        let mut file = try!(File::open(&wad_path).in_file(&wad_path));
        let header = try!(file.read_binary::<WadInfo>().in_file(&wad_path));
        try!(wad_type_from_info(&header).ok_or_else(|| BadWadHeader.in_file(&wad_path)));

        // Read lump info.
        let mut lumps = Vec::with_capacity(header.num_lumps as usize);
        let mut levels = Vec::with_capacity(64);
        let mut index_map = HashMap::new();

        try!(file.seek(SeekFrom::Start(header.info_table_offset as u64)).in_file(&wad_path));
        for i_lump in 0 .. header.num_lumps {
            let mut fileinfo = try!(file.read_binary::<WadLump>().in_file(&wad_path));
            fileinfo.name.canonicalise();
            index_map.insert(fileinfo.name, lumps.len());
            lumps.push(LumpInfo { name: fileinfo.name,
                                  offset: fileinfo.file_pos as u64,
                                  size: fileinfo.size as usize });

            // Our heuristic for level lumps is that they are preceeded by the "THINGS" lump.
            if fileinfo.name == b"THINGS\0\0".to_wad_name() {
                assert!(i_lump > 0);
                levels.push((i_lump - 1) as usize);
            }
        }

        // Read metadata.
        let meta = try!(WadMetadata::from_file(meta_path));

        Ok(Archive {
            meta: meta,
            file: RefCell::new(file),
            lumps: lumps,
            index_map: index_map,
            levels: levels,
            path: wad_path,
        })
    }

    pub fn num_levels(&self) -> usize { self.levels.len() }

    pub fn level_lump_index(&self, level_index: usize) -> usize {
        self.levels[level_index]
    }

    pub fn level_name(&self, level_index: usize) -> &WadName {
        self.lump_name(self.levels[level_index])
    }

    pub fn num_lumps(&self) -> usize { self.lumps.len() }

    pub fn named_lump_index(&self, name: &WadName) -> Option<usize> {
        self.index_map.get(name).map(|x| *x)
    }

    pub fn required_named_lump_index(&self, name: &WadName) -> Result<usize> {
        self.named_lump_index(name).ok_or(MissingRequiredLump(*name)).in_archive(self)
    }

    pub fn lump_name(&self, lump_index: usize) -> &WadName {
        &self.lumps[lump_index].name
    }

    pub fn is_virtual_lump(&self, lump_index: usize) -> bool {
        self.lumps[lump_index].size == 0
    }

    pub fn read_required_named_lump<T: Copy>(&self, name: &WadName) -> Result<Vec<T>> {
        self.read_named_lump(name)
            .unwrap_or_else(|| Err(MissingRequiredLump(*name).in_archive(self)))
    }

    pub fn read_named_lump<T: Copy>(&self, name: &WadName) -> Option<Result<Vec<T>>> {
        self.named_lump_index(name).map(|index| self.read_lump(index))
    }

    pub fn read_lump<T: Copy>(&self, index: usize) -> Result<Vec<T>> {
        let mut file = self.file.borrow_mut();
        let info = self.lumps[index];
        assert!(info.size > 0);
        assert!(info.size % mem::size_of::<T>() == 0);
        let num_elems = info.size / mem::size_of::<T>();
        let mut buf = Vec::with_capacity(num_elems);
        try!(file.seek(SeekFrom::Start(info.offset)).in_archive(self));
        unsafe {
            buf.set_len(num_elems);
            try!(file.read_at_least(slice::from_raw_parts_mut(
                    (buf.as_mut_ptr() as *mut u8), info.size)).in_archive(self))
        }
        Ok(buf)
    }

    pub fn read_lump_single<T: Copy>(&self, index: usize) -> Result<T> {
        let mut file = self.file.borrow_mut();
        let info = self.lumps[index];
        assert!(info.size == mem::size_of::<T>());
        try!(file.seek(SeekFrom::Start(info.offset)).in_archive(self));
        Ok(try!(file.read_binary().in_archive(self)))
    }

    pub fn metadata(&self) -> &WadMetadata { &self.meta }
}

pub trait InArchive {
    type Output;
    fn in_archive(self, archive: &Archive) -> Self::Output;
}

impl InArchive for Error {
    type Output = Error;
    fn in_archive(self, archive: &Archive) -> Error {
        self.in_file(&archive.path)
    }
}

impl InArchive for ErrorKind {
    type Output = Error;
    fn in_archive(self, archive: &Archive) -> Error {
        self.in_file(&archive.path)
    }
}

impl<S, E: Into<Error>> InArchive for StdResult<S, E> {
    type Output = Result<S>;
    fn in_archive(self, archive: &Archive) -> Result<S> {
        self.map_err(|e| e.into().in_archive(archive))
    }
}


#[derive(Copy, Clone)]
struct LumpInfo {
    name: WadName,
    offset: u64,
    size: usize,
}
