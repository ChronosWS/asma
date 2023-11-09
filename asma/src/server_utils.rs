use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use regex::Regex;

use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{ChildStdout, Command},
    sync::mpsc::Sender,
};
use tracing::{error, trace, warn};
use uuid::Uuid;

use crate::AsyncNotification;

#[derive(Debug, Clone)]
pub enum UpdateMode {
    Update,
    Validate,
}

#[derive(Debug, Clone)]
pub enum UpdateServerProgress {
    Initializing,
    Downloading(f32),
    Verifying(f32),
}

pub async fn start_server(
    server_id: Uuid,
    server_name: impl AsRef<str>,
    installation_dir: impl AsRef<str>,
    map: impl AsRef<str>,
    port: u16,
) -> Result<u32> {
    let installation_dir = installation_dir.as_ref();
    let exe = Path::new(installation_dir)
        .join("ShooterGame/Binaries/Win64/ArkAscendedServer.exe")
        .canonicalize()
        .expect("Failed to canonicalize path");

    let _profile_descriptor = format!("\"ASA.{}.{}\"", server_id.to_string(), server_name.as_ref());
    let args = [
        //&format!("\"ASA.{}.{}\"", server_id.to_string(), server_name.as_ref()),
        //exe.to_str().expect("Failed to convert path to string"),
        //"/NORMAL",
        &format!("{}?Port={}", map.as_ref(), port),
    ];

    // If we want to tag the process with metadata, we either need to force set the title after launch,
    // or run it via a batch file using `start "<profile_descriptor>"` ...
    let mut command = Command::new(exe);
    command.args(args);
    let command_string = format!("{:?}", command);
    let child = command
        .spawn()
        .map_err(|e| {
            error!("Spawn failed: {}", e.to_string());
            e
        })
        .with_context(|| format!("Failed to spawn server: {}", command_string))?;
    let pid = child.id().expect("Failed to get child process id");
    Ok(pid)
}

pub async fn update_server(
    server_id: Uuid,
    steamcmd_dir: impl AsRef<str>,
    installation_dir: impl AsRef<str>,
    app_id: impl AsRef<str>,
    mode: UpdateMode,
    progress: Sender<AsyncNotification>,
) -> Result<()> {
    let steamcmd_dir = steamcmd_dir.as_ref();
    let installation_dir = installation_dir.as_ref();
    let steamcmd_exe = Path::new(steamcmd_dir).join("steamcmd.exe");

    let mut args = vec![
        "+force_install_dir",
        installation_dir,
        "+login",
        "anonymous",
    ];

    match mode {
        UpdateMode::Update => {
            args.push("+app_update");
            args.push(app_id.as_ref())
        }
        UpdateMode::Validate => {
            args.push("validate");
        }
    }

    args.push("+quit");

    // let command = args.join(" ");

    // let mut proc = conpty::spawn(&command)?;
    // let reader = proc.output().unwrap();

    // let mut line_reader = std::io::BufReader::with_capacity(32, reader);

    let mut command = Command::new(steamcmd_exe);

    command.args(args);
    command.stdout(Stdio::piped());

    let mut child = command.spawn()?;
    let stdout: ChildStdout = child.stdout.take().expect("Failed to get piped stdout");

    let progress_parser = Regex::new(
        r"Update state \(0x(?<state>[0-9a-fA-F]+)\) (?<desc>[^,]*), progress: (?<percent>[0-9.]+)",
    )
    .expect("Failed to compile progress regex");

    let line_reader = BufReader::new(stdout);
    let mut lines = line_reader.lines();

    let _ = progress
        .send(AsyncNotification::UpdateServerProgress(
            server_id,
            UpdateServerProgress::Initializing,
        ))
        .await;
    //Update state (0x61) downloading, progress: 99.76 (9475446175 / 9498529183)
    //Update state (0x81) verifying update, progress: 7.18 (681966749 / 9498529183)

    // HACK: SteamCMD is an ill-behaved piece of software which makes it difficult to grab progress line-by-line.
    // See: https://github.com/ValveSoftware/Source-1-Games/issues/1684

    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                if let Some(captures) = progress_parser.captures(&line) {
                    if captures.len() == 4 {
                        let state = captures.name("state").expect("Failed to get state");
                        let desc = captures.name("desc").expect("Failed to get desc");
                        let percent = captures.name("percent").expect("Failed to get percent");

                        let state = u64::from_str_radix(state.as_str(), 16)
                            .expect("Failed to parse status code");
                        let percent: f32 =
                            percent.as_str().parse().expect("Failed to parse prpogress");

                        match state {
                            0x61 => {
                                trace!("{}: SteamCMD: Downloading {}", server_id, percent);
                                let _ = progress
                                    .send(AsyncNotification::UpdateServerProgress(
                                        server_id,
                                        UpdateServerProgress::Downloading(percent),
                                    ))
                                    .await;
                            }
                            0x81 => {
                                trace!("{}: SteamCMD: Verifying {}", server_id, percent);
                                let _ = progress
                                    .send(AsyncNotification::UpdateServerProgress(
                                        server_id,
                                        UpdateServerProgress::Verifying(percent),
                                    ))
                                    .await;
                            }
                            other => {
                                warn!(
                                    "{}: SteamCMD: Unknown state: {} ({})",
                                    server_id,
                                    other,
                                    desc.as_str()
                                )
                            }
                        }
                    }
                } else {
                    trace!("{}: SteamCMD: {}", server_id, &line);
                }
            }
            Ok(None) => {
                break;
            }
            Err(e) => {
                error!(
                    "{}: SteamCMD: Error reading output: {}",
                    server_id,
                    e.to_string()
                );
                break;
            }
        }
    }

    child
        .wait()
        .await
        .map(|_| ())
        .with_context(|| "steam_cmd failed")
}

#[derive(Debug, Clone)]
pub enum ValidationResult {
    NotInstalled,
    Success(String),
    Failed(String),
}

const STATE_INSTALL_SUCCESSFUL: u32 = 4;

pub async fn validate_server(
    id: Uuid,
    installation_dir: impl AsRef<str>,
    app_id: impl AsRef<str>,
) -> Result<ValidationResult> {
    // Verify the binary exists
    let installation_dir = installation_dir.as_ref();
    let base_path = PathBuf::from(installation_dir);

    // Validate install state
    let manifest_path = base_path.join(format!("steamapps/appmanifest_{}.acf", app_id.as_ref()));
    match std::fs::read_to_string(manifest_path) {
        Err(err) => match err.kind() {
            ErrorKind::NotFound => {
                trace!("{}: No appmanifest found", id);
                return Ok(ValidationResult::NotInstalled);
            }
            _ => return Err(err.into()),
        },
        Ok(content) => {
            let regex = Regex::new("StateFlags[^0-9]+(?<state>[0-9]+)")
                .expect("Failed to build manifest searching regex");
            let state_capture = content.lines().filter_map(|l| regex.captures(l)).next();
            if let Some(state_capture) = state_capture {
                let state_flags: u32 = state_capture
                    .name("state")
                    .expect("Failed to get named capture")
                    .as_str()
                    .parse()
                    .with_context(|| "state flags failed to parse")?;
                if state_flags != STATE_INSTALL_SUCCESSFUL {
                    trace!("{}: Incomplete install (state = {})", id, state_flags);
                    return Ok(ValidationResult::Failed("Incomplete".to_string()));
                }
            } else {
                trace!("{}: appmanifest does not contain state information", id);
                return Ok(ValidationResult::NotInstalled);
            }
        }
    }

    // Validate binary path
    let binary_path = base_path.join("ShooterGame/Binaries/Win64/ArkAscendedServer.exe");
    let metadata = match std::fs::metadata(binary_path) {
        Ok(metadata) => metadata,
        Err(err) => match err.kind() {
            ErrorKind::NotFound => {
                trace!("{}: No binary found", id);
                return Ok(ValidationResult::NotInstalled);
            }
            _ => return Err(err.into()),
        },
    };

    let created_time: DateTime<Local> =
        DateTime::from(metadata.created().with_context(|| "No Creation Time")?);
    Ok(ValidationResult::Success(created_time.to_rfc3339()))
}
