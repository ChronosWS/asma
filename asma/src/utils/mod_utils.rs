use std::path::PathBuf;

use crate::{
    models::{get_default_curseforge_app_id, Server},
    server::{ModUpdateRecords, ServerModsRecord},
    AsyncNotification,
};
use anyhow::{Context, Result};
use curseforge::{prelude::ClientOptions, Client};
use iter_tools::*;
use tokio::sync::mpsc::Sender;
use tracing::{trace, warn};
use uuid::Uuid;

static PROXY_API_BASE: &str = "https://api.curse.tools/v1/cf/";
static CLIENT_OPTIONS: ClientOptions = ClientOptions {
    // This is the maximum number of client connections allowed for the host.
    // Increasing this number may result in denial errors.
    max_connections: 1,
};

#[derive(Clone, Debug)]
pub enum ModStatus {
    UpToDate,
    OutOfDate,
    Removed,
}

#[derive(Clone, Debug)]
pub struct ServerModsStatus {
    pub server_id: Uuid,
    pub mod_statuses: Vec<(i32, ModStatus)>,
}

#[derive(Clone, Debug)]
pub struct ServerModsStatuses {
    pub server_statuses: Vec<ServerModsStatus>,
}

struct InstalledMod {
    server_id: Uuid,
    project_id: i32,
    file_id: i32,
}

pub fn get_mod_update_records(servers: &Vec<Server>) -> ModUpdateRecords {
    ModUpdateRecords {
        servers: servers
            .iter()
            .map(|s| ServerModsRecord {
                server_id: s.id(),
                installation_dir: s.settings.installation_location.to_owned(),
                mod_ids: s.settings.get_mod_ids(),
            })
            .collect(),
    }
}

pub async fn check_for_mod_updates<'a>(
    status_sender: &Sender<AsyncNotification>,
    mod_update_records: &ModUpdateRecords,
) -> Result<()> {
    trace!("Checking for mod updates");
    // First, start with all of the records with no file_id (mod version)
    let mut requested_mods = mod_update_records
        .servers
        .iter()
        .flat_map(|s| {
            s.mod_ids.iter().map(|m| InstalledMod {
                server_id: s.server_id,
                project_id: *m,
                file_id: 0,
            })
        })
        .collect::<Vec<InstalledMod>>();

    // Now, for each requested_mod, find the corresponding installed mod, if it exists
    for mods_record in mod_update_records.servers.iter() {
        let mut mods_dir = PathBuf::from(&mods_record.installation_dir);
        mods_dir.push("ShooterGame");
        mods_dir.push("Binaries");
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
                    if let Some(requested_mod) = requested_mods.iter_mut().find(|m| {
                        m.server_id == mods_record.server_id && m.project_id == installed_mod[0]
                    }) {
                        // Update the version in the requested_mods record
                        requested_mod.file_id = installed_mod[1];
                    }
                }
            }
        } else {
            warn!("Failed to read mods directory {}", mods_dir.display())
        }
    }

    if requested_mods.is_empty() {
        trace!("Skipping mods update check - no mods configured");
        return Ok(());
    }

    // Now query curseforge on the set of unique mods we want versions for
    let unique_project_ids: Vec<i32> = requested_mods
        .iter()
        .map(|m| m.project_id)
        .unique()
        .collect();

    let client = Client::new(PROXY_API_BASE, None, Some(&CLIENT_OPTIONS)).unwrap();
    let projects = client
        .projects(unique_project_ids)
        .await
        .with_context(|| "Failed to get project statuses")?;

    // Finally, compare the versions returned from the api with the versions we have installed
    let mut mods_statuses: Vec<ServerModsStatus> = Vec::new();
    for requested_mod in requested_mods.iter() {
        // Get the mod status
        let mod_status =
            if let Some(project) = projects.iter().find(|p| p.id == requested_mod.project_id) {
                if project.main_file_id > requested_mod.file_id {
                    // There is an update available
                    trace!(
                        "Server {} Mod {} is out of date",
                        requested_mod.server_id,
                        requested_mod.project_id
                    );
                    (requested_mod.project_id, ModStatus::OutOfDate)
                } else {
                    // No update needed
                    trace!(
                        "Server {} Mod {} is up-to-date",
                        requested_mod.server_id,
                        requested_mod.project_id
                    );
                    (requested_mod.project_id, ModStatus::UpToDate)
                }
            } else {
                // Installed mod has been remove from CurseForge
                warn!(
                    "Server {} Mod {} is no longer available",
                    requested_mod.server_id, requested_mod.project_id
                );
                (requested_mod.project_id, ModStatus::Removed)
            };

        // Update the status record
        if let Some(server_status) = mods_statuses
            .iter_mut()
            .find(|s| s.server_id == requested_mod.server_id)
        {
            server_status.mod_statuses.push(mod_status);
        } else {
            mods_statuses.push(ServerModsStatus {
                server_id: requested_mod.server_id,
                mod_statuses: vec![mod_status],
            });
        }
    }

    // Send the status update
    let _ = status_sender
        .send(AsyncNotification::ServerModsStatuses(ServerModsStatuses {
            server_statuses: mods_statuses,
        }))
        .await;

    Ok(())
}
