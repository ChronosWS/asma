use std::{
    fs::File,
    io::Read,
    io::{BufWriter, Write},
    path::Path,
};

use anyhow::{Result, bail};
use chrono::prelude::*;
use serde::Serialize;
use sha2::{Digest, Sha256};
use vergen::EmitBuilder;
use zip::write::*;

#[derive(Serialize)]
struct DefaultConfigManifest {
    hash: String,
    date: DateTime<Utc>,
}

enum ReleaseTarget {
    Dev,
    Rel
}

fn main() -> Result<()> {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let release_target = std::env::var("RELEASE_TARGET").unwrap_or("dev".into());
    let release_target = match release_target.to_ascii_lowercase().as_str() {
        "dev" => ReleaseTarget::Dev,
        "rel" => ReleaseTarget::Rel,
        _ =>  bail!("Invalid release target specified!")
    };

    write_default_config_manifest(out_dir);
    write_version_json(out_dir);

    EmitBuilder::builder().all_build().emit()?;
    if let ReleaseTarget::Rel = release_target{
        println!("cargo:rustc-env=IS_RELEASE_TARGET=true");
    }
    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}

#[derive(Serialize)]
struct Version {
    version: String,
}

fn write_version_json(out_dir: &Path) {
    let version_path = out_dir.join("version.json");
    let version = env!("CARGO_PKG_VERSION").to_owned();

    serde_json::to_writer(
        BufWriter::new(File::create(version_path).expect("Failed to create version file")),
        &Version { version },
    )
    .expect("Failed to serialize version");
}

fn write_default_config_manifest(out_dir: &Path) {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let default_config_path = Path::new(manifest_dir)
        .join("res")
        .join("data")
        .join("default_config_metadata.json");
    println!(
        "Looking for default config in {}",
        default_config_path.display()
    );
    println!("cargo:rerun-if-changed={}", default_config_path.display());
    let default_config = std::fs::read_to_string(default_config_path).unwrap();
    let default_config_output_path = Path::new(&out_dir).join("default_config_metadata.json.zip");
    println!(
        "Zip file output to {}",
        default_config_output_path.display()
    );

    {
        let mut zip_writer =
            zip::write::ZipWriter::new(std::fs::File::create(&default_config_output_path).unwrap());
        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o755);
        zip_writer
            .start_file("default_config_metadata.json", options)
            .unwrap();
        zip_writer.write_all(default_config.as_bytes()).unwrap();
        zip_writer.finish().unwrap();
    }

    let mut zip_bytes = Vec::new();
    std::fs::File::open(&default_config_output_path)
        .unwrap()
        .read_to_end(&mut zip_bytes)
        .unwrap();
    let mut hasher = Sha256::new();
    hasher.update(&zip_bytes);
    let config_hash = hex::encode(hasher.finalize());

    let default_config_manifest_output_path =
        Path::new(&out_dir).join("default_config_manifest.json");

    let default_config_manifest = DefaultConfigManifest {
        hash: config_hash,
        date: Utc::now(),
    };
    std::fs::write(
        default_config_manifest_output_path,
        serde_json::to_string_pretty(&default_config_manifest).unwrap(),
    )
    .unwrap();
}
