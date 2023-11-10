use std::path::PathBuf;

use anyhow::Result;
use iced::{Font, font::{Family, Weight, Stretch}};
use tracing::trace;

pub const BOLD_FONT: Font = Font {
    family: Family::Name("Arial"),
    weight: Weight::Bold,
    stretch: Stretch::Normal,
    style: iced::font::Style::Normal
};

pub fn get_system_font_bytes(font_file: &str) -> Result<Vec<u8>> {
    let system_dir =
        std::env::var("SystemRoot").expect("Failed to get SystemRoot environment variable");
    let path: PathBuf = [system_dir.as_str(), "fonts", font_file].iter().collect(); 
   
    let bytes = std::fs::read(&path)?;
    trace!("Loaded {} bytes from font file {:?}", bytes.len(), &path);
    Ok(bytes)
}
