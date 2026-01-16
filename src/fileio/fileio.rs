use std::fs;

use serde::{Serialize, de::DeserializeOwned};

pub fn save_struct_to_file<T: Serialize>(obj: &T, path: &str) {
    let text = serde_json::to_string_pretty(obj).expect("Failed serialising object");

    fs::write(path, text).expect(&format!("Failed writing text to path {}", path))
}

pub fn load_struct_from_file<T: DeserializeOwned>(path: &str) -> T {
    let data = fs::read_to_string(path).expect(&format!("Failed reading file at path {}", path));

    serde_json::from_str::<T>(&data).expect("Failed deserialising object")
}

/// Delete all files in the given directory except files whose name contains ignore_filter.
pub fn clear_directory(path: &str, ignore_filter: &str) {
    let files = fs::read_dir(path).expect(&format!("Couldn't read directory {path}"));

    for file in files {
        let file_info = file.unwrap();

        if !file_info.path().is_file() || file_info.file_name().to_str().unwrap().contains(ignore_filter) {
            continue;
        }

        fs::remove_file(file_info.path()).expect(&format!("Failed to delete file at path {}", file_info.path().display()));
    }
}
