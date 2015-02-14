#![feature(collections, io, path)]

use std::old_io::fs::File;
use std::string::String;

pub fn read_utf8_file(path: &Path) -> Result<String, String> {
    File::open(path)
        .and_then(|mut file| file.read_to_end())
        .map_err(|e| String::from_str(e.desc))
        .and_then(|buffer| {
            String::from_utf8(buffer).map_err(|_| {
                format!("File at '{}' is not valid UTF-8.", path.display())
            })
        })
}

pub fn vec_from_elem<T: Clone>(len: usize, elem: T) -> Vec<T> {
    <Vec<T> as std::iter::FromIterator<T>>::from_iter(
        std::iter::repeat(elem).take(len))
}
