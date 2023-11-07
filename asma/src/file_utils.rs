pub fn directory_exists(path: impl AsRef<str>) -> bool {
    if let Ok(metadata) = std::fs::metadata(path.as_ref()) {
        metadata.is_dir()
    } else {
        false
    }
}
