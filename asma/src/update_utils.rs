use std::{
    fs::File,
    io::{Cursor, ErrorKind},
    process::{exit, Command},
    thread::sleep,
};

use anyhow::{Context, Result};
use reqwest::Url;
use rfd::MessageDialogResult;
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

#[cfg(feature = "win2016")]
mod release_files {
    pub const LATEST_REL_VERSION: &str = "latest-rel.win2016.json";
    pub const LATEST_DEV_VERSION: &str = "latest-dev.win2016.json";
    pub const LATEST_REL_ZIP: &str = "latest-rel.win2016.zip";
    pub const LATEST_DEV_ZIP: &str = "latest-dev.win2016.zip";
}

#[cfg(not(feature = "win2016"))]
mod release_files {
    pub const LATEST_REL_VERSION: &str = "latest-rel.json";
    pub const LATEST_DEV_VERSION: &str = "latest-dev.json";
    pub const LATEST_REL_ZIP: &str = "latest-rel.json";
    pub const LATEST_DEV_ZIP: &str = "latest-dev.json";
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
                .and(Some(release_files::LATEST_REL_ZIP))
                .unwrap_or(release_files::LATEST_DEV_ZIP),
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
                .and(Some(release_files::LATEST_REL_VERSION))
                .unwrap_or(release_files::LATEST_DEV_VERSION),
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

    loop {
        let mut iterations = 10usize;
        while iterations > 0 {
            if let Err(e) = std::fs::copy(&asma_new_exe_path, &asma_exe_path) {
                warn!(
                    "Couldn't copy {} to {}: {}",
                    asma_new_exe_path.display(),
                    asma_exe_path.display(),
                    e.to_string()
                );
                sleep(std::time::Duration::from_secs(2));
                iterations -= 1;
            } else {
                break;
            }
        }

        if iterations > 0 {
            if let Err(e) = Command::new(&asma_exe_path).spawn() {
                rfd::MessageDialog::new()
                    .set_title("Failed to restart ASMA")
                    .set_description(format!(
                        "Failed to restart {}: {}. Check the path restart it (also report this issue).",
                        asma_exe_path.display(), 
                        e.to_string()
                    ))
                    .set_level(rfd::MessageLevel::Warning)
                    .set_buttons(rfd::MessageButtons::Ok)
                    .show();
                exit(-1);
            } else {
                exit(0);
            }
        } else {
            error!("Failed to copy asma.exe");
            let result = rfd::MessageDialog::new()
                .set_title("Self-update failed!")
                .set_description(
                format!("Could not copy {} to {}.  Check that asma.exe has shut down and that {} is a writeable path. Retry?",
                    asma_new_exe_path.display(),
                    asma_exe_path.display(),
                    asma_exe_path.display()))
                .set_buttons(rfd::MessageButtons::YesNo)
                .set_level(rfd::MessageLevel::Error)
                .show();

            if let MessageDialogResult::Yes = result {
                continue;
            } else {
                exit(-1);
            }
        }
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
