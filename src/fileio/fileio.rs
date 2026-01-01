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
