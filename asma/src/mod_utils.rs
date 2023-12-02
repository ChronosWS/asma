use std::path::PathBuf;

use crate::{models::get_default_curseforge_app_id, server::ServerMods, AsyncNotification};
use anyhow::{Context, Result};
use curseforge::{prelude::ClientOptions, Client};
use iter_tools::*;
use tokio::sync::mpsc::Sender;
use tracing::warn;
use uuid::Uuid;

static PROXY_API_BASE: &str = "https://api.curse.tools/v1/cf/";
static CLIENT_OPTIONS: ClientOptions = ClientOptions {
    // This is the maximum number of client connections allowed for the host.
    // Increasing this number may result in denial errors.
    max_connections: 1,
};

pub enum ModStatus {
    UpToDate,
    OutOfDate,
    Removed,
}

pub struct ServerModsStatus {
    server_id: Uuid,
    mod_statuses: Vec<(i32, ModStatus)>,
}

pub struct ServerModsStatuses {
    pub server_statuses: Vec<ServerModsStatus>,
}

struct InstalledMod {
    server_id: Uuid,
    project_id: i32,
    file_id: i32,
}
pub async fn check_for_mod_updates(
    status_sender: &Sender<AsyncNotification>,
    installation_dirs: Vec<(Uuid, &str)>,
) -> Result<()> {
    // Determine the set of installed mods
    let mut installed_mods = Vec::new();
    for (server_id, installation_dir) in installation_dirs.iter() {
        let mut mods_dir = PathBuf::from(installation_dir);
        mods_dir.push("ShooterGame");
        mods_dir.push("Binarier");
        mods_dir.push("Win64");
        mods_dir.push("ShooterGame");
        mods_dir.push("Mods");
        mods_dir.push(get_default_curseforge_app_id());

        if let Ok(dir_entries) = std::fs::read_dir(&mods_dir) {
            for dir_entry in dir_entries
                .filter(|e| e.is_ok())
                .map(|e| e.unwrap())
                .map(|e| e.file_name().to_str().map(|s| s.to_owned()))
                .filter(|e| e.is_some())
                .map(|e| e.unwrap())
            {
                let dir_entry = dir_entry.split('_');
                let installed_mod = dir_entry
                    .map(|s| s.parse::<i32>().unwrap_or_default())
                    .filter(|&v| v > 0)
                    .collect::<Vec<_>>();
                if installed_mod.len() == 2 {
                    installed_mods.push(InstalledMod {
                        server_id: *server_id,
                        project_id: installed_mod[0],
                        file_id: installed_mod[1],
                    })
                }
            }
        } else {
            warn!("Failed to read mods directory {}", mods_dir.display())
        }
    }

    let unique_mods: Vec<i32> = installed_mods
        .iter()
        .map(|e| e.project_id)
        .unique()
        .collect();

    let client = Client::new(PROXY_API_BASE, None, Some(&CLIENT_OPTIONS)).unwrap();
    let projects = client
        .projects(unique_mods)
        .await
        .with_context(|| "Failed to get project statuses")?;

    let mut mods_statuses: Vec<ServerModsStatus> = Vec::new();
    for installed_mod in installed_mods.iter() {
        let mod_status =
            if let Some(project) = projects.iter().find(|p| p.id == installed_mod.project_id) {
                if project.main_file_id > installed_mod.file_id {
                    // There is an update available
                    (installed_mod.project_id, ModStatus::OutOfDate)
                } else {
                    // No update needed
                    (installed_mod.project_id, ModStatus::UpToDate)
                }
            } else {
                // Installed mod has been remove from CurseForge
                (installed_mod.project_id, ModStatus::Removed)
            };

        if let Some(server_status) = mods_statuses
            .iter_mut()
            .find(|s| s.server_id == installed_mod.server_id)
        {
            server_status.mod_statuses.push(mod_status);
        } else {
            mods_statuses.push(ServerModsStatus {
                server_id: installed_mod.server_id,
                mod_statuses: vec![mod_status],
            });
        }
    }

    let _ = status_sender
        .send(AsyncNotification::ServerModsStatuses(ServerModsStatuses {
            server_statuses: mods_statuses,
        }))
        .await;

    Ok(())
}
