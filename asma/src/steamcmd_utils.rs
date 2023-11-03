use anyhow::{bail, Context, Result};
use futures_util::StreamExt;
use std::{io::Write, path::PathBuf};
use tracing::{error, trace};

// TODO: magic strings
pub async fn get_steamcmd<S: AsRef<str>>(destination_path: S) -> Result<()> {
    let destination_path = destination_path.as_ref();
    trace!("Getting steamcmd to {}", destination_path);
    let mut zip_file_name = PathBuf::from(destination_path);
    zip_file_name.push("steamcmd.zip");

    let mut file = std::fs::File::create(zip_file_name.as_path()).with_context(|| {
        format!(
            "Failed to open archive file {} for writing",
            zip_file_name.to_str().unwrap_or_default()
        )
    })?;
    let mut response_stream =
        reqwest::get("https://steamcdn-a.akamaihd.net/client/installer/steamcmd.zip")
            .await
            .with_context(|| "Failed to get steamcmd from remote host")?
            .bytes_stream();

    while let Some(bytes) = response_stream.next().await {
        let bytes = bytes.with_context(|| "Failed to read bytes from stream")?;
        let bytes_written = file
            .write(bytes.as_ref())
            .with_context(|| format!("Failed to write bytes to {}", destination_path))?;
        if bytes_written != bytes.len() {
            bail!("Wrote {}, expected {}", bytes_written, bytes.len());
        }
    }

    trace!("steamcmd downloaded, unzipping");

    let file = std::fs::File::open(zip_file_name.as_path()).with_context(|| {
        format!(
            "Failed to open archive file {} for reading",
            zip_file_name.to_str().unwrap_or_default()
        )
    })?;

    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| {
            error!("Failed to read zip archive: {}", e.to_string());
            e
        })
        .with_context(|| {
            format!(
                "Failed to read zip archive {}",
                zip_file_name.to_str().unwrap_or_default()
            )
        })?;

    archive
        .extract(destination_path)
        .with_context(|| format!("Failed to extract zip archive to {destination_path}"))?;

    trace!("steamcmd unzipped");
    let mut steamcmd_exe = PathBuf::from(destination_path);
    steamcmd_exe.push("steamcmd.exe");

    std::fs::File::open(steamcmd_exe.as_path()).with_context(|| "Failed to verify steamcmd.exe")?;
    Ok(())
}
