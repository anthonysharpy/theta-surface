use std::fs;

use serde::{Serialize, de::DeserializeOwned};

use crate::types::TSError;
use crate::types::TSErrorType::RuntimeError;

pub fn save_struct_to_file<T: Serialize>(obj: &T, path: &str) -> Result<(), TSError> {
    let text =
        serde_json::to_string_pretty(obj).map_err(|e| TSError::new(RuntimeError, format!("Failed serialising object: {}", e)))?;

    fs::write(path, text).map_err(|e| TSError::new(RuntimeError, format!("Failed writing text to path {}: {}", path, e)))?;

    Ok(())
}

pub fn load_struct_from_file<T: DeserializeOwned>(path: &str) -> Result<T, TSError> {
    let data = fs::read_to_string(path)
        .map_err(|e| TSError::new(RuntimeError, format!("Failed reading file at path {}: {}", path, e)))?;

    Ok(
        serde_json::from_str::<T>(&data)
            .map_err(|e| TSError::new(RuntimeError, format!("Failed deserialising object: {}", e)))?,
    )
}

/// Delete all files in the given directory except files whose name contains ignore_filter.
pub fn clear_directory(path: &str, ignore_filter: &str) -> Result<(), TSError> {
    let files = fs::read_dir(path).map_err(|e| TSError::new(RuntimeError, format!("Coulnd't read directory {path}: {e}")))?;

    for file in files {
        let file_info = file.map_err(|e| TSError::new(RuntimeError, format!("File reference was invalid: {e}")))?;
        let path = file_info.path();
        let path_name = path.display();
        let raw_file_name = file_info.file_name();
        let file_name = raw_file_name
            .to_str()
            .ok_or(TSError::new(RuntimeError, "Failed getting file name"))?;

        if !file_info.path().is_file() || file_name.contains(ignore_filter) {
            continue;
        }

        fs::remove_file(file_info.path())
            .map_err(|e| TSError::new(RuntimeError, format!("Failed to delete file at path {path_name}: {e}")))?;
    }

    Ok(())
}
