
use std::fs;

pub fn file_list(directory: &str) -> Vec<String> {
    let paths = fs::read_dir(directory).unwrap();
    let mut result = vec![];
    for path in paths {
        let path_str = path.unwrap().file_name().to_str().unwrap().to_string();
        result.push(path_str);
    }
    result
}

// println!("Name: {}", path_str);
// let data = fs::read_to_string(path_str).expect("Something's wrong with the file.");
// println!("{}", data);
