#![feature(collections, io, fs, path)]

use std::fs::File;
use std::io::{self, Read};
use std::path::AsPath;

pub fn read_utf8_file<P: AsPath>(path: &P) -> Result<String, String> {
    File::open(path)
        .and_then(|mut file| {
            let mut buffer = vec![];
            file.read_to_end(&mut buffer).map(|_| buffer)
        })
        .map_err(|e| String::from_str(e.description()))
        .and_then(|buffer| {
            String::from_utf8(buffer).map_err(|_| {
                format!("File at '{:?}' is not valid UTF-8.", path.as_path())
            })
        })
}

pub fn read_at_least<R: Read>(from: &mut R, mut buf: &mut [u8])
        -> io::Result<()> {
    if buf.len() == 0 { return Ok(()); }
    let len = try!(from.read(buf));
    read_at_least(from, &mut buf[len..])
}

pub fn vec_from_elem<T: Clone>(len: usize, elem: T) -> Vec<T> {
    <Vec<T> as std::iter::FromIterator<T>>::from_iter(
        std::iter::repeat(elem).take(len))
}
