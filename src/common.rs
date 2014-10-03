use std::io::fs::File;
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
