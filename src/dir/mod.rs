
use std::fs;
use crate::STATIC_DIR;

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