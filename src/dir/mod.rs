use crate::STATIC_DIR;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

pub fn file_list() -> Vec<String> {
    let dir_lock = STATIC_DIR.read().unwrap();
    let dir = &*dir_lock;
    let paths = fs::read_dir(dir).unwrap();
    let mut result = vec![];
    for path in paths {
        let path_str = path.unwrap().file_name().to_str().unwrap().to_string();
        result.push(path_str);
    }
    result
}

// To avoid over-writing already existing files.
pub fn generate_file_address(file_name: &str, sr: bool) -> String {
    let static_dir = &*STATIC_DIR.read().unwrap();
    let buf_immut = PathBuf::new().join(static_dir).join(file_name);
    let mut display_str = String::from(buf_immut.to_str().unwrap());
    // This stupid duplication is the only way I could get away with
    // cloning a Path. JESUS 'EFFIN CHRIST
    if sr {
        let mut file_path_buf = PathBuf::new().join(static_dir).join(file_name);
        let file_extension = buf_immut.extension().unwrap_or(OsStr::new(".txt"));
        file_path_buf.set_extension("");
        let file_name = file_path_buf.file_name().unwrap();
        let mut lossy_string = file_name.to_string_lossy();
        let b = lossy_string.to_mut();
        b.push_str("-1");
        let file_path_buf = PathBuf::new().join(static_dir);
        let mut file_path_buf = file_path_buf.join(b);
        file_path_buf.set_extension(file_extension);
        display_str = String::from(Path::new(&file_path_buf).to_str().unwrap());
    }
    // let file_path = file_path_buf.as_path();
    display_str
}
