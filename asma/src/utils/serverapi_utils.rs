use std::{io::{Cursor, ErrorKind}, path::{PathBuf, Path}};

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use tokio::sync::mpsc::Sender;
use tracing::trace;
use zip::ZipArchive;

use crate::{update_utils::StandardVersion, AsyncNotification, models::ServerApiState};

#[derive(Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
    content_type: String,
}

#[derive(Deserialize)]
struct GithubRelease {
    name: String,
    assets: Vec<ReleaseAsset>,
}

#[derive(Default, Debug, Clone)]
pub struct ServerApiVersion {
    pub version: StandardVersion,
    pub download_url: String,
}

pub async fn check_for_server_api_updates(
    status_sender: &Sender<AsyncNotification>,
    server_api_update_url: impl AsRef<str>,
) -> Result<()> {
    let client = reqwest::Client::new();
    let releases: Vec<GithubRelease> = client
        .get(server_api_update_url.as_ref())
        .header("User-Agent", "Ark Server Manager Ascended")
        .send()
        .await
        .with_context(|| "Failed to create ServerApi request")?
        .json()
        .await
        .with_context(|| "Failed to deserialize ServerAPI releases")?;

    let mut latest_release = None;
    for release in releases.iter() {
        let version = StandardVersion::new(&release.name);
        if latest_release
            .as_ref()
            .map(|(_, latest_version, _)| version > *latest_version)
            .unwrap_or(true)
        {
            if let Some((asset_index, _)) = release.assets.iter().enumerate().find(|(_, asset)| {
                asset.content_type == "application/x-zip-compressed"
                    && asset.name == format!("AsaApi_{}.zip", release.name)
            }) {
                latest_release = Some((release, version, asset_index))
            }
        }
    }

    if let Some((release, version, asset_index)) = latest_release {
        trace!("Latest ServerApi version is {}", version);
        let _ = status_sender
            .send(AsyncNotification::ServerApiVersion(ServerApiVersion {
                version,
                download_url: release.assets[asset_index].browser_download_url.to_owned(),
            }))
            .await;
    }
    Ok(())
}

pub fn check_server_api_install_state(install_location: impl AsRef<str>) -> ServerApiState {
    let base_path = Path::new(install_location.as_ref());
    let server_api_version_path =
        base_path.join("ShooterGame/Binaries/Win64/server_api_version.json");
    if let Ok(version) = std::fs::File::open(server_api_version_path)
        .and_then(|f| {
            serde_json::from_reader(f).map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e))
        }) {
        ServerApiState::Installed { version }
    } else {
        ServerApiState::NotInstalled
    }
}

pub async fn install_server_api(
    server_api_version: ServerApiVersion,
    install_location: impl AsRef<str>,
) -> Result<()> {
    let client = reqwest::Client::new();
    let bytes_stream = client
        .get(&server_api_version.download_url)
        .header("User-Agent", "Ark Server Manager Ascended")
        .send()
        .await
        .with_context(|| "Failed to create ServerApi request")?
        .bytes()
        .await
        .with_context(|| "Failed to download ServerApi")?;

    trace!("Read {} bytes", bytes_stream.len());
    let mut install_path = PathBuf::from(install_location.as_ref());
    install_path.push("ShooterGame");
    install_path.push("Binaries");
    install_path.push("Win64");

    // Extract from the archive
    let buf_reader = Cursor::new(&bytes_stream[..]);
    let mut zip_archive = match ZipArchive::new(buf_reader) {
        Ok(archive) => archive,
        Err(e) => bail!("Failed to open archive: {}", e.to_string()),
    };
    zip_archive
        .extract(&install_path)
        .with_context(|| format!("Failed to extract archive to {}", install_path.display()))?;

    install_path.push("server_api_version.json");
    serde_json::to_writer(
        std::fs::File::create(&install_path).with_context(|| "Failed to create version.json")?,
        &server_api_version.version,
    )
    .with_context(|| "Failed to serialize version")?;
    trace!("ServerApi installed to {}", install_path.display());
    Ok(())
}

pub fn remove_server_api(
    install_location: impl AsRef<str>
) -> Result<()> {
    let mut install_path = PathBuf::from(install_location.as_ref());
    install_path.push("ShooterGame");
    install_path.push("Binaries");
    install_path.push("Win64");
    install_path.push("server_api_version.json");
    std::fs::remove_file(&install_path).with_context(|| format!("Failed to remove {}", install_path.display()))
}