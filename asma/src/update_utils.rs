use std::{
    fs::File,
    io::{Cursor, ErrorKind},
    process::{exit, Command},
    thread::sleep,
};

use anyhow::{Context, Result};
use reqwest::Url;
use serde::Deserialize;
use std::io::{Read, Write};
use tokio::sync::mpsc::Sender;
use tracing::{error, trace, warn};
use zip::ZipArchive;

use crate::AsyncNotification;

#[derive(Debug, Clone)]
pub enum AsmaUpdateState {
    CheckingForUpdates,
    AvailableVersion(String),
    Downloading,
    UpdateReady,
    UpdateFailed,
}

pub async fn update_asma(
    status_sender: &Sender<AsyncNotification>,
    app_update_url: &Url,
) -> Result<()> {
    let _ = status_sender
        .send(AsyncNotification::AsmaUpdateState(
            AsmaUpdateState::Downloading,
        ))
        .await;

    let url = app_update_url
        .join(
            option_env!("IS_RELEASE_TARGET")
                .and(Some("latest-rel.zip"))
                .unwrap_or("latest-dev.zip"),
        )
        .with_context(|| "Failed to parse update url")?;

    // Download the new version
    let response = reqwest::get(url)
        .await
        .with_context(|| "Failed to get update")?;
    let bytes_stream = response
        .bytes()
        .await
        .with_context(|| "Failed to download latest.zip")?;

    let mut asma_new_exe_path =
        process_path::get_executable_path().with_context(|| "Failed to get process path")?;
    asma_new_exe_path.set_file_name("asma.new.exe");

    // Extract from the archive
    let buf_reader = Cursor::new(&bytes_stream[..]);
    let mut zip_archive =
        ZipArchive::new(buf_reader).with_context(|| "Failed to open archive from stream")?;
    let mut asma_exe_result = zip_archive
        .by_name("asma.exe")
        .with_context(|| "Failed to find asma.exe in zip archive")?;
    let mut buf = Vec::new();
    asma_exe_result
        .read_to_end(&mut buf)
        .with_context(|| "Failed to read asma.exe")?;
    File::create(&asma_new_exe_path)
        .with_context(|| "Failed to create asma.new.exe")?
        .write_all(&buf)
        .with_context(|| "Failed to write asma.new.exe")?;

    Command::new(asma_new_exe_path)
        .args(["--do-update"])
        .spawn()
        .with_context(|| "Failed to spawn update")?;

    Ok(())
}

pub async fn check_for_asma_updates(
    status_sender: &Sender<AsyncNotification>,
    app_update_url: &Url,
) -> Result<()> {
    // Check for ASMA updates
    let url = app_update_url
        .join(
            option_env!("IS_RELEASE_TARGET")
                .and(Some("latest-rel.json"))
                .unwrap_or("latest-dev.json"),
        )
        .with_context(|| "Failed to parse update url")?;
    let version_response = reqwest::get(url)
        .await
        .with_context(|| "Failed to get latest version")?;

    #[derive(Deserialize)]
    struct Version {
        version: String,
    }

    let version: Version = version_response
        .json()
        .await
        .with_context(|| "Failed to deserialize version information")?;

    let _ = status_sender
        .send(AsyncNotification::AsmaUpdateState(
            AsmaUpdateState::AvailableVersion(version.version),
        ))
        .await;
    Ok(())
}

pub fn restart() -> ! {
    trace!("Exiting to perform update");
    exit(0);
}

pub fn do_update() -> ! {
    let asma_exe_path = process_path::get_executable_path().expect("Failed to get process path");
    let mut asma_new_exe_path = asma_exe_path.clone();
    asma_new_exe_path.set_file_name("asma.new.exe");

    let mut iterations = 10usize;
    while iterations > 0 && std::fs::copy(&asma_new_exe_path, &asma_exe_path).is_err() {
        sleep(std::time::Duration::from_secs(2));
        iterations -= 1;
    }

    if iterations > 0 {
        Command::new(&asma_exe_path)
            .spawn()
            .expect("Failed to respawn ASMA.exe");
        exit(0);
    } else {
        error!("Failed to copy asma.exe");
        exit(-1);
    }
}

pub fn cleanup_update() {
    let mut asma_new_exe_path =
        process_path::get_executable_path().expect("Failed to get process path");
    asma_new_exe_path.set_file_name("asma.new.exe");

    let mut iterations = 10usize;
    while iterations > 0 {
        if let Err(e) = std::fs::remove_file(&asma_new_exe_path) {
            if let ErrorKind::NotFound = e.kind() {
                trace!("No {} found to clean up", asma_new_exe_path.display());
                return;
            } 
        } else {
            trace!("Cleaned up {}", asma_new_exe_path.display());
            return;
        }
        sleep(std::time::Duration::from_secs(2));
        iterations -= 1;
    }

    warn!("Cleanup failed");
}
