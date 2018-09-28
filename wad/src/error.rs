use std::fmt::Debug;

use error_chain::error_chain;

error_chain! {
    foreign_links {}
    errors {
        CorruptMetadata(message: String) {
            description("Corrupt metadata file.")
                display("Corrupt metadata file: {}", message)
        }
        CorruptWad(message: String) {
            description("Corrupt WAD file.")
            display("Corrupt WAD file: {}", message)
        }
        Io(message: String) {
            description("I/O WAD error.")
            display("I/O WAD error: {}", message)
        }
    }
    links {}
}

impl ErrorKind {
    pub fn invalid_byte_in_wad_name(byte: u8, bytes: &[u8]) -> ErrorKind {
        ErrorKind::CorruptWad(format!(
            "Invalid character `{}` in wad name `{}`.",
            char::from(byte),
            String::from_utf8_lossy(bytes),
        ))
    }

    pub fn wad_name_too_long(bytes: &[u8]) -> ErrorKind {
        ErrorKind::CorruptWad(format!(
            "Wad name too long `{}`.",
            String::from_utf8_lossy(bytes)
        ))
    }

    pub fn bad_wad_header() -> ErrorKind {
        ErrorKind::CorruptWad("Could not read WAD header.".to_owned())
    }

    pub fn bad_wad_header_identifier(identifier: &[u8]) -> ErrorKind {
        ErrorKind::CorruptWad(format!(
            "Invalid header identifier: {}",
            String::from_utf8_lossy(identifier)
        ))
    }

    pub fn on_metadata_read() -> ErrorKind {
        ErrorKind::Io("Failed to load metadata to memory.".to_owned())
    }

    pub fn on_file_open() -> ErrorKind {
        ErrorKind::Io("Failed to open file.".to_owned())
    }

    pub fn bad_lump_info(lump_index: i32) -> ErrorKind {
        ErrorKind::CorruptWad(format!("Invalid lump info for lump {}", lump_index))
    }

    pub fn bad_lump_element(lump_index: usize, lump_name: &str, element_index: usize) -> ErrorKind {
        ErrorKind::CorruptWad(format!(
            "Invalid element {} in lump `{}` (index={})",
            element_index, lump_name, lump_index
        ))
    }

    pub fn bad_lump_size(
        index: usize,
        name: &str,
        total_size: usize,
        element_size: usize,
    ) -> ErrorKind {
        ErrorKind::CorruptWad(format!(
            "Invalid lump size in `{}` (index={}): total={}, element={}, div={}, mod={}",
            name,
            index,
            total_size,
            element_size,
            total_size / element_size,
            total_size % element_size
        ))
    }

    pub fn missing_required_lump<N: Debug>(name: &N) -> ErrorKind {
        ErrorKind::CorruptWad(format!("Missing required lump {:?}", name))
    }

    pub fn seeking_to_info_table_offset(offset: i32) -> ErrorKind {
        ErrorKind::Io(format!(
            "Seeking to `info_table_offset` at {} failed",
            offset
        ))
    }

    pub fn seeking_to_lump(index: usize, name: &str) -> ErrorKind {
        ErrorKind::Io(format!("Seeking to lump {}, `{}` failed", index, name))
    }

    pub fn reading_lump(index: usize, name: &str) -> ErrorKind {
        ErrorKind::Io(format!("Reading lump {}, `{}` failed", index, name))
    }
}
