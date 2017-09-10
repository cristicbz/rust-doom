use super::error::{ErrorKind, Result, ResultExt};
use super::meta::WadMetadata;
use super::types::{WadInfo, WadLump, WadName};
use bincode::{deserialize_from as bincode_read, Infinite};
use ordermap::OrderMap;
use serde::de::DeserializeOwned;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::fmt::Debug;
use std::fs::File;
use std::hash::Hash;
use std::io::{BufReader, Seek, SeekFrom};
use std::mem;
use std::path::Path;
use std::vec::Vec;


pub struct Archive {
    file: RefCell<BufReader<File>>,
    index_map: OrderMap<WadName, usize>,
    lumps: Vec<LumpInfo>,
    levels: Vec<usize>,
    meta: WadMetadata,
}

impl Archive {
    pub fn open<W, M>(wad_path: &W, meta_path: &M) -> Result<Archive>
    where
        W: AsRef<Path> + Debug,
        M: AsRef<Path> + Debug,
    {
        let wad_path = wad_path.as_ref().to_owned();
        let meta_path = meta_path.as_ref().to_owned();
        info!("Loading wad file '{:?}'...", wad_path);
        let (file, index_map, lumps, levels) = Archive::open_wad(&wad_path)?;
        info!("Loading metadata file '{:?}'...", meta_path);
        let meta = WadMetadata::from_file(&meta_path)?;

        Ok(Archive {
            file,
            meta,
            lumps,
            index_map,
            levels,
        })
    }

    fn open_wad(
        wad_path: &Path,
    ) -> Result<(RefCell<BufReader<File>>, OrderMap<WadName, usize>, Vec<LumpInfo>, Vec<usize>)> {
        // Open file, read and check header.
        let mut file = BufReader::new(File::open(&wad_path).chain_err(
            || ErrorKind::on_file_open(),
        )?);

        let header: WadInfo = bincode_read(&mut file, Infinite).chain_err(|| {
            ErrorKind::bad_wad_header()
        })?;

        ensure!(
            &header.identifier == IWAD_HEADER,
            ErrorKind::bad_wad_header_identifier(&header.identifier)
        );

        // Read lump info.
        let mut lumps = Vec::with_capacity(header.num_lumps as usize);
        let mut levels = Vec::with_capacity(64);
        let mut index_map = OrderMap::new();

        file.seek(SeekFrom::Start(header.info_table_offset as u64))
            .chain_err(|| {
                ErrorKind::seeking_to_info_table_offset(header.info_table_offset)
            })?;
        for i_lump in 0..header.num_lumps {
            let fileinfo: WadLump = bincode_read(&mut file, Infinite).chain_err(|| {
                ErrorKind::bad_lump_info(i_lump)
            })?;

            index_map.insert(fileinfo.name, lumps.len());
            lumps.push(LumpInfo {
                name: fileinfo.name,
                offset: fileinfo.file_pos as u64,
                size: fileinfo.size as usize,
            });

            // Our heuristic for level lumps is that they are preceeded by the "THINGS"
            // lump.
            if &fileinfo.name == b"THINGS\0\0" {
                assert!(i_lump > 0);
                levels.push((i_lump - 1) as usize);
            }
        }

        Ok((RefCell::new(file), index_map, lumps, levels))
    }

    pub fn num_levels(&self) -> usize {
        self.levels.len()
    }

    pub fn level_lump_index(&self, level_index: usize) -> usize {
        self.levels[level_index]
    }

    pub fn level_name(&self, level_index: usize) -> &WadName {
        self.lump_name(self.levels[level_index])
    }

    pub fn num_lumps(&self) -> usize {
        self.lumps.len()
    }

    pub fn named_lump_index<Q>(&self, name: &Q) -> Option<usize>
    where
        WadName: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.index_map.get(name).cloned()
    }

    pub fn required_named_lump_index<Q>(&self, name: &Q) -> Result<usize>
    where
        WadName: Borrow<Q>,
        Q: Hash + Eq + Debug,
    {
        self.named_lump_index(name).ok_or_else(|| {
            ErrorKind::missing_required_lump(name).into()
        })
    }

    pub fn lump_name(&self, lump_index: usize) -> &WadName {
        &self.lumps[lump_index].name
    }

    pub fn is_virtual_lump(&self, lump_index: usize) -> bool {
        self.lumps[lump_index].size == 0
    }

    pub fn read_required_named_lump<Q, T>(&self, name: &Q) -> Result<Vec<T>>
    where
        WadName: Borrow<Q>,
        T: DeserializeOwned,
        Q: Hash + Eq + Debug,
    {
        self.required_named_lump_index(name).and_then(|index| {
            self.read_lump(index)
        })
    }

    pub fn read_named_lump<Q, T>(&self, name: &Q) -> Option<Result<Vec<T>>>
    where
        WadName: Borrow<Q>,
        T: DeserializeOwned,
        Q: Hash + Eq,
    {
        self.named_lump_index(name).map(
            |index| self.read_lump(index),
        )
    }

    pub fn read_lump<T: DeserializeOwned>(&self, index: usize) -> Result<Vec<T>> {
        let mut file_guard = self.file.borrow_mut();
        let mut file = &mut *file_guard;
        let info = &self.lumps.get(index).ok_or_else(|| {
            ErrorKind::missing_required_lump(&index)
        })?;
        let element_size = mem::size_of::<T>();
        let num_elements = info.size / element_size;

        ensure!(
            info.size > 0 && (info.size % element_size == 0),
            ErrorKind::bad_lump_size(index, info.name.as_ref(), info.size, element_size)
        );

        file.seek(SeekFrom::Start(info.offset)).chain_err(|| {
            ErrorKind::seeking_to_lump(index, info.name.as_ref())
        })?;

        (0..num_elements)
            .map(move |i_element| {
                bincode_read(file, Infinite).chain_err(|| {
                    ErrorKind::bad_lump_element(index, info.name.as_ref(), i_element)
                })
            })
            .collect()
    }


    pub fn read_lump_single<T: DeserializeOwned>(&self, index: usize) -> Result<T> {
        let element_size = mem::size_of::<T>();
        let mut file = self.file.borrow_mut();
        let info = &self.lumps.get(index).ok_or_else(|| {
            ErrorKind::missing_required_lump(&index)
        })?;
        ensure!(
            info.size > 0 && (info.size == element_size),
            ErrorKind::bad_lump_size(index, info.name.as_ref(), info.size, element_size)
        );
        file.seek(SeekFrom::Start(info.offset)).chain_err(|| {
            ErrorKind::seeking_to_lump(index, info.name.as_ref())
        })?;
        Ok(bincode_read(&mut *file, Infinite).chain_err(|| {
            ErrorKind::bad_lump_element(index, info.name.as_ref(), 0)
        })?)
    }

    pub fn metadata(&self) -> &WadMetadata {
        &self.meta
    }
}

#[derive(Copy, Clone)]
struct LumpInfo {
    name: WadName,
    offset: u64,
    size: usize,
}

const IWAD_HEADER: &'static [u8] = b"IWAD";
