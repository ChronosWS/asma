use std::collections::HashMap;

use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use serde::Deserialize;
use tokio::sync::mpsc::Sender;
use tracing::trace;

use crate::AsyncNotification;

#[derive(Deserialize)]
struct SteamAppBranch {
    buildid: String,
    timeupdated: String,
}

#[derive(Deserialize)]
struct SteamAppBranches {
    public: SteamAppBranch,
}

#[derive(Deserialize)]
struct SteamAppDepot {
    branches: SteamAppBranches,
}

#[derive(Deserialize)]
struct SteamAppInfo {
    // _change_number: u64,
    // _missing_token: bool,
    // _sha: String,
    // _size: u64,
    // appid: String,
    // common: SteamAppCommon,
    // config: SteamAppConfig,
    depots: SteamAppDepot,
}

#[derive(Deserialize)]
struct SteamAppInfoResponse {
    data: HashMap<String, SteamAppInfo>,
    //status: String,
}

#[derive(Default, Debug, Clone)]
pub struct SteamAppVersion {
    pub buildid: u64,
    pub timeupdated: DateTime<Local>,
}

pub async fn check_for_steam_updates(
    status_sender: &Sender<AsyncNotification>,
    steam_app_id: &str,
) -> Result<()> {
    trace!("Checking for server updates");
    let response = reqwest::get(format!("https://api.steamcmd.net/v1/info/{}", steam_app_id))
        .await
        .with_context(|| "Web request failed")?
        .bytes()
        .await
        .with_context(|| "Failed to get body stream")?;

    let response: SteamAppInfoResponse =
        serde_json::from_slice(&response[..]).with_context(|| "Failed to deserialize response")?;

    let app_info = response
        .data
        .get(steam_app_id)
        .with_context(|| format!("Failed to get app info for {}", steam_app_id))?;

    let _ = status_sender.send(AsyncNotification::SteamAppUpdate(SteamAppVersion {
        buildid: app_info
            .depots
            .branches
            .public
            .buildid
            .parse()
            .unwrap_or_default(),
        timeupdated: DateTime::from_timestamp(
            app_info
                .depots
                .branches
                .public
                .timeupdated
                .parse()
                .unwrap_or_default(),
            0,
        )
        .unwrap_or_default()
        .into(),
    })).await;
    Ok(())
}
