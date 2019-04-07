use super::errors::{ErrorKind, Result};
use super::meta::WadMetadata;
use super::name::IntoWadName;
use super::types::{WadInfo, WadLump, WadName};
use bincode;
use failchain::{ensure, ResultExt};
use indexmap::IndexMap;
use log::info;
use serde::de::DeserializeOwned;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::fmt::Debug;
use std::fs::File;
use std::hash::Hash;
use std::io::{BufReader, Read, Seek, SeekFrom, Take};
use std::mem;
use std::path::Path;
use std::vec::Vec;

#[derive(Debug)]
pub struct Archive {
    file: RefCell<BufReader<File>>,
    index_map: IndexMap<WadName, usize>,
    lumps: Vec<LumpInfo>,
    levels: Vec<usize>,
    meta: WadMetadata,
}

struct OpenWad {
    file: RefCell<BufReader<File>>,
    index_map: IndexMap<WadName, usize>,
    lumps: Vec<LumpInfo>,
    levels: Vec<usize>,
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
        let OpenWad {
            file,
            index_map,
            lumps,
            levels,
        } = Archive::open_wad(&wad_path)?;
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

    fn open_wad(wad_path: &Path) -> Result<OpenWad> {
        // Open file, read and check header.
        let mut file = BufReader::new(File::open(&wad_path).chain_err(ErrorKind::on_file_open)?);

        let header: WadInfo =
            bincode::deserialize_from(&mut file).chain_err(ErrorKind::bad_wad_header)?;

        ensure!(
            header.identifier == IWAD_HEADER,
            ErrorKind::bad_wad_header_identifier(&header.identifier)
        );

        // Read lump info.
        let mut lumps = Vec::with_capacity(header.num_lumps as usize);
        let mut levels = Vec::with_capacity(64);
        let mut index_map = IndexMap::new();

        file.seek(SeekFrom::Start(header.info_table_offset as u64))
            .chain_err(|| ErrorKind::seeking_to_info_table_offset(header.info_table_offset))?;
        for i_lump in 0..header.num_lumps {
            let fileinfo: WadLump = bincode::deserialize_from(&mut file)
                .chain_err(|| ErrorKind::bad_lump_info(i_lump))?;

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

        Ok(OpenWad {
            file: RefCell::new(file),
            index_map,
            lumps,
            levels,
        })
    }

    pub fn metadata(&self) -> &WadMetadata {
        &self.meta
    }

    pub fn num_levels(&self) -> usize {
        self.levels.len()
    }

    pub fn level_lump(&self, level_index: usize) -> Result<LumpReader> {
        self.lump_by_index(self.levels[level_index])
    }

    pub fn required_named_lump<'a, Q>(&self, name: &'a Q) -> Result<LumpReader>
    where
        &'a Q: IntoWadName,
    {
        let name: WadName = name.into_wad_name()?;
        self.named_lump(&name)?
            .ok_or_else(|| ErrorKind::missing_required_lump(&name).into())
    }

    pub fn named_lump<Q>(&self, name: &Q) -> Result<Option<LumpReader>>
    where
        WadName: Borrow<Q>,
        Q: Hash + Eq,
    {
        match self.index_map.get(name) {
            Some(&index) => self.lump_by_index(index).map(Some),
            None => Ok(None),
        }
    }

    pub fn lump_by_index(&self, index: usize) -> Result<LumpReader> {
        Ok(LumpReader {
            archive: self,
            info: self
                .lumps
                .get(index)
                .ok_or_else(|| ErrorKind::missing_required_lump(&index))?,
            index,
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub struct LumpReader<'a> {
    archive: &'a Archive,
    info: &'a LumpInfo,
    index: usize,
}

impl<'a> LumpReader<'a> {
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn name(&self) -> WadName {
        self.info.name
    }

    pub fn is_virtual(&self) -> bool {
        self.info.size == 0
    }

    pub fn decode_vec<T: DeserializeOwned>(&self) -> Result<Vec<T>> {
        let LumpReader { info, index, .. } = *self;
        self.read(|mut file| {
            let element_size = mem::size_of::<T>();
            let num_elements = info.size / element_size;

            ensure!(
                info.size > 0 && (info.size % element_size == 0),
                ErrorKind::bad_lump_size(index, info.name.as_ref(), info.size, element_size),
            );
            (0..num_elements)
                .map(move |i_element| {
                    bincode::deserialize_from(&mut file).chain_err(|| {
                        ErrorKind::bad_lump_element(index, info.name.as_ref(), i_element)
                    })
                })
                .collect()
        })
    }

    pub fn decode_one<T: DeserializeOwned>(&self) -> Result<T> {
        let LumpReader { info, index, .. } = *self;
        self.read(|file| {
            let element_size = mem::size_of::<T>();
            ensure!(
                element_size > 0 && info.size == element_size,
                ErrorKind::bad_lump_size(index, info.name.as_ref(), info.size, element_size)
            );
            Ok(bincode::deserialize_from(file)
                .chain_err(|| ErrorKind::bad_lump_element(index, info.name.as_ref(), 0))?)
        })
    }

    pub fn read_blobs<B>(&self) -> Result<Vec<B>>
    where
        B: Default + AsMut<[u8]>,
    {
        let LumpReader { info, index, .. } = *self;
        self.read(|file| {
            let blob_size = B::default().as_mut().len();
            assert!(blob_size > 0);
            ensure!(
                info.size > 0 && (info.size % blob_size) == 0,
                ErrorKind::bad_lump_size(index, info.name.as_ref(), info.size, blob_size),
            );
            let num_blobs = info.size / blob_size;
            let mut blobs = Vec::with_capacity(num_blobs);
            for _ in 0..num_blobs {
                blobs.push(B::default());
                file.read_exact(blobs.last_mut().expect("empty blobs").as_mut())
                    .chain_err(|| ErrorKind::reading_lump(index, info.name.as_ref()))?;
            }
            Ok(blobs)
        })
    }

    pub fn read_bytes_into(&self, bytes: &mut Vec<u8>) -> Result<()> {
        let LumpReader { info, index, .. } = *self;
        self.read(|file| {
            let old_size = bytes.len();
            bytes.resize(old_size + info.size, 0u8);
            file.read_exact(&mut bytes[old_size..])
                .chain_err(|| ErrorKind::reading_lump(index, info.name.as_ref()))?;
            Ok(())
        })
    }

    pub fn read_bytes(&self) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        self.read_bytes_into(&mut bytes).map(|_| bytes)
    }

    fn read<F, T>(&self, with: F) -> Result<T>
    where
        F: FnOnce(&mut Take<&mut BufReader<File>>) -> Result<T>,
    {
        let LumpReader {
            info,
            index,
            archive,
        } = *self;
        let mut file = archive.file.borrow_mut();
        file.seek(SeekFrom::Start(info.offset))
            .chain_err(|| ErrorKind::seeking_to_lump(index, info.name.as_ref()))?;
        with(&mut Read::take(&mut *file, info.size as u64))
    }
}

#[derive(Copy, Clone, Debug)]
struct LumpInfo {
    name: WadName,
    offset: u64,
    size: usize,
}

const IWAD_HEADER: &[u8] = b"IWAD";
