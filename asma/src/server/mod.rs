use anyhow::{bail, Context, Result};
use chrono::{DateTime, Local};
use ini::Ini;
use regex::Regex;

use std::{
    collections::HashMap,
    fs::File,
    io::{ErrorKind, Read},
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

use crate::{
    config_utils::ConfigMetadataState,
    models::{
        config::{
            ConfigLocation, ConfigMetadata, ConfigQuantity, ConfigValue, ConfigValueBaseType,
            ConfigValueType, ConfigVariant, IniFile,
        },
        ServerSettings,
    },
    AsyncNotification,
};

mod monitor;

pub use monitor::{monitor_server, ServerMonitorCommand};

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

pub fn generate_command_line(
    config_metadata: &ConfigMetadataState,
    server_settings: &ServerSettings,
) -> Result<Vec<String>> {
    let mut args: Vec<String> = Vec::new();

    let config_metadata = config_metadata.effective();
    // Map metadata to each entry
    let settings_meta_map: Vec<_> = server_settings
        .config_entries
        .entries
        .iter()
        .map(|e| {
            config_metadata
                .find_entry(&e.meta_name, &e.meta_location)
                .and_then(|(_, m)| {
                    Some((e, m)).or_else(|| {
                        error!(
                            "Failed to find metadata entry {} {} for config entry",
                            e.meta_name, e.meta_location
                        );
                        None
                    })
                })
        })
        .filter(|v| v.is_some())
        .map(Option::unwrap)
        .collect();

    if settings_meta_map.len() < server_settings.config_entries.entries.len() {
        bail!("One or more config entries did not have a metadata mapping")
    }

    let map = if let Some(map) = settings_meta_map
        .iter()
        .find(|(e, _)| e.meta_location == ConfigLocation::MapName)
        .map(|(e, _)| e.value.to_string())
    {
        Some(map)
    } else if let Some(map) = config_metadata
        .entries
        .iter()
        .find(|e| e.location == ConfigLocation::MapName)
        .map(|e| e.default_value.as_ref().map(|e| e.to_string()))
    {
        map
    } else {
        None
    }
    .with_context(|| "Failed to find required MapName setting")?;

    let url_params = settings_meta_map
        .iter()
        .filter(|(e, _)| e.meta_location == ConfigLocation::MapUrlOption)
        .map(|(e, _)| format!("{}={}", e.meta_name, e.value.to_string()))
        .collect::<Vec<_>>()
        .join("?");

    let switch_params = settings_meta_map
        .iter()
        .filter(|(e, _)| e.meta_location == ConfigLocation::CommandLineOption)
        .map(|(e, m)| {
            if let ConfigValueType {
                quantity: ConfigQuantity::Scalar,
                base_type: ConfigValueBaseType::Bool,
            } = m.value_type
            {
                if let ConfigVariant::Scalar(ConfigValue::Bool(b)) = e.value {
                    if b {
                        format!("-{}", e.meta_name)
                    } else {
                        String::new()
                    }
                } else {
                    error!(
                        "Config entry {} actual type doesn't match metadata type",
                        e.meta_name
                    );
                    String::new()
                }
            } else {
                format!("-{}={}", e.meta_name, e.value.to_string())
            }
        })
        .map(|s| s.into());

    if url_params.is_empty() {
        args.push(map.to_owned());
    } else {
        args.push(format!("{}?{}", map, url_params).into());
    }

    args.extend(switch_params);

    Ok(args)
}

pub fn update_inis_from_settings(
    _config_metadata: &ConfigMetadata,
    server_settings: &ServerSettings,
) -> Result<()> {
    let installation_dir = server_settings.get_full_installation_location();
    let ini_settings = server_settings
        .config_entries
        .entries
        .iter()
        .map(|e| {
            if let ConfigLocation::IniOption(file, section) = &e.meta_location {
                Some((file, section, e))
            } else {
                None
            }
        })
        .filter(Option::is_some)
        .map(Option::unwrap)
        .collect::<Vec<_>>();

    fn ini_file_name(installation_dir: &str, file: &IniFile) -> PathBuf {
        Path::new(installation_dir)
            .join("ShooterGame/Saved/Config/WindowsServer")
            .join(file.to_string())
            .with_extension("ini")
            .canonicalize()
            .expect("Failed to canonicalize path")
    }

    let mut ini_files = HashMap::new();
    for (file, section, entry) in ini_settings {
        match ini_files
            .entry(file)
            .or_insert_with(|| Ini::load_from_file(ini_file_name(&installation_dir, file)))
        {
            Ok(ini) => {
                trace!(
                    "Setting {}:[{}] {} = {}",
                    file.to_string(),
                    section.to_string(),
                    entry.meta_name,
                    entry.value
                );
                ini.set_to(
                    Some(section.to_string()),
                    entry.meta_name.to_owned(),
                    entry.value.to_string(),
                );
            }
            Err(e) => bail!("Failed to load ini file: {}", e.to_string()),
        }
    }

    for (file, ini_result) in ini_files.drain() {
        if let Ok(ini) = ini_result {
            let file_name = ini_file_name(&installation_dir, file);
            trace!("Writing INI file {}", file_name.display());
            ini.write_to_file(&file_name)
                .with_context(|| format!("Failed to write ini file {}", file_name.display()))?;
        }
    }

    Ok(())
}

/// Starts the server, returns the PID of the running process
pub async fn start_server(
    server_id: Uuid,
    server_name: impl AsRef<str>,
    installation_dir: impl AsRef<str>,
    args: Vec<String>,
) -> Result<()> {
    let installation_dir = installation_dir.as_ref();
    let exe = Path::new(installation_dir)
        .join("ShooterGame/Binaries/Win64/ArkAscendedServer.exe")
        .canonicalize()
        .expect("Failed to canonicalize path");

    let _profile_descriptor = format!("\"ASA.{}.{}\"", server_id.to_string(), server_name.as_ref());

    // If we want to tag the process with metadata, we either need to force set the title after launch,
    // or run it via a batch file using `start "<profile_descriptor>"` ...
    let mut command = Command::new(exe);
    command.args(args);
    let command_string = format!("{:?}", command);
    trace!("Launching server: {}", command_string);
    let child = command
        .spawn()
        .map_err(|e| {
            error!("Spawn failed: {}", e.to_string());
            e
        })
        .with_context(|| format!("Failed to spawn server: {}", command_string))?;
    let pid = child.id().expect("Failed to get child process id");
    trace!("{}: PID: {}", server_id, pid);
    Ok(())
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

// NOTE: PERFORMANCE: This algorithm works reasonably, but can take several seconds on debug builds.
fn get_asa_version(exe_path: &PathBuf) -> Result<String> {
    let file = std::fs::File::open(exe_path)?;
    let mut reader = std::io::BufReader::new(file);

    // The string "ArkVersion" represented as Unicode, as it exists in the binary
    // NOTE: The algorithm used here is NOT general-purpose across any kind of target bytes
    let target_bytes = [
        0x41, 0x00, 0x72, 0x00, 0x6B, 0x00, 0x56, 0x00, 0x65, 0x00, 0x72, 0x00, 0x73, 0x00, 0x69,
        0x00, 0x6F, 0x00, 0x6E, 0x00, 0x00, 0x00,
    ];

    fn read_to_byte(reader: &mut std::io::BufReader<File>, needle: u8) -> bool {
        loop {
            let mut actual_byte = [0u8];
            if reader.read_exact(&mut actual_byte).is_ok() {
                if actual_byte[0] == needle {
                    return true;
                }
            } else {
                return false;
            }
        }
    }

    let mut bytes_read = Vec::new();
    loop {
        bytes_read.clear();
        if read_to_byte(&mut reader, target_bytes[0]) {
            let result = target_bytes[1..]
                .iter()
                .enumerate()
                .find_map(|(index, &needle)| {
                    let mut actual_byte = [0u8];
                    if reader.read_exact(&mut actual_byte).is_ok() && actual_byte[0] == needle {
                        bytes_read.push(actual_byte[0]);
                        None
                    } else {
                        Some(index)
                    }
                });
            match result {
                Some(_) => {}
                None => {
                    break;
                }
            }
        } else {
            error!("End of file looking for version string");
            return Ok(String::new());
        }
    }

    let mut version = String::new();
    let mut buf = [0u8; 2];
    while reader.read_exact(&mut buf).is_ok() {
        let unicode_val = u16::from_le_bytes(buf);
        if unicode_val == 0 {
            break;
        }
        if let Some(char) = char::from_u32(unicode_val as u32) {
            version.push(char);
        } else {
            error!("ERROR: Failed to convert character");
            break;
        }
    }

    Ok(version)
}

#[derive(Debug, Clone)]
pub enum ValidationResult {
    NotInstalled,
    Success { version: String, install_time: DateTime<Local> },
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
    let metadata = match std::fs::metadata(&binary_path) {
        Ok(metadata) => metadata,
        Err(err) => match err.kind() {
            ErrorKind::NotFound => {
                trace!("{}: No binary found", id);
                return Ok(ValidationResult::NotInstalled);
            }
            _ => return Err(err.into()),
        },
    };

    // Find the version in the binary
    let version = get_asa_version(&binary_path)?;

    let install_time: DateTime<Local> =
        DateTime::from(metadata.created().with_context(|| "No Creation Time")?);
    Ok(ValidationResult::Success { version, install_time })
}