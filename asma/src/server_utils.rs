use anyhow::{Context, Result};
use regex::Regex;
use std::{path::Path, process::Stdio};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{ChildStdout, Command},
    sync::mpsc::Sender,
};
use tracing::{error, trace, warn};
use uuid::Uuid;

use crate::AsyncNotification;

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
        "+app_update",
        app_id.as_ref(),
    ];
    if let UpdateMode::Validate = mode {
        args.push("validate");
    }

    args.push("+quit");

    // let command = args.join(" ");

    // let mut proc = conpty::spawn(&command)?;
    // let reader = proc.output().unwrap();

    // let mut line_reader = std::io::BufReader::with_capacity(32, reader);

    let mut command = Command::new(steamcmd_exe);

    command.args(args);
    command.stdout(Stdio::piped());
    command.kill_on_drop(true);

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
                                trace!("Downloading {}", percent);
                                let _ = progress
                                    .send(AsyncNotification::UpdateServerProgress(
                                        server_id,
                                        UpdateServerProgress::Downloading(percent),
                                    ))
                                    .await;
                            }
                            0x81 => {
                                trace!("Verifying {}", percent);
                                let _ = progress
                                    .send(AsyncNotification::UpdateServerProgress(
                                        server_id,
                                        UpdateServerProgress::Verifying(percent),
                                    ))
                                    .await;
                            }
                            other => {
                                warn!("Unknown steamcmd state: {} ({})", other, desc.as_str())
                            }
                        }
                    }
                } else {
                    trace!("Line: {}", &line);
                }
            }
            Ok(None) => {
                trace!("Stream ended");
                break;
            }
            Err(e) => {
                error!("Error reading output: {}", e.to_string());
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
    Success(String),
    Failed(String)
}

pub async fn validate_server(id: Uuid, installation_dir: impl AsRef<str>) -> Result<ValidationResult> {
    Ok(ValidationResult::Failed("Not implemented".into()))
}