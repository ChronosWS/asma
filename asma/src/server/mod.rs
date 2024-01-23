use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use iter_tools::Itertools;
use regex::Regex;

use std::{
    fs::File,
    io::{ErrorKind, Read},
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::{process::Command, task::yield_now, time::Instant};

use tracing::{error, trace, warn};
use uuid::Uuid;

use crate::{
    config_utils::{import_ini_with_metadata, ConfigMetadataState},
    models::{
        config::{
            ConfigEntries, ConfigLocation, ConfigMetadata, ConfigQuantity, ConfigValue,
            ConfigValueBaseType, ConfigValueType, ConfigVariant,
        },
        ServerApiState, ServerSettings,
    },
    serverapi_utils::check_server_api_install_state,
};

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

pub(crate) fn import_server_settings(
    config_metadata: &ConfigMetadata,
    installation_location: PathBuf,
    import_inis: bool,
) -> Result<ServerSettings> {
    let mut config_entries = ConfigEntries::default();

    if import_inis {
        let mut ini_path = installation_location.join("ShooterGame/Saved/Config/WindowsServer/foo");
        for file_name in config_metadata
            .entries
            .iter()
            .map(|e| &e.location)
            .filter_map(|l| {
                if let ConfigLocation::IniOption(file_name, _) = l {
                    Some(file_name.clone())
                } else {
                    None
                }
            })
            .unique()
        {
            ini_path.set_file_name(file_name.to_string());
            ini_path.set_extension("ini");
            trace!("Importing from {}", ini_path.display());

            if let Ok(mut imported_config_entries) =
                import_ini_with_metadata(config_metadata, &ini_path)
            {
                config_entries
                    .entries
                    .append(&mut imported_config_entries.entries);
            }
        }
    }

    let server_settings = ServerSettings {
        id: Uuid::new_v4(),
        name: installation_location
            .file_name()
            .expect("Failed to get filename")
            .to_str()
            .expect("Failed to convert file name to string")
            .to_owned(),
        installation_location: installation_location
            .to_str()
            .expect("Failed to convert path to string")
            .to_owned(),
        allow_external_ini_management: !import_inis,
        use_external_rcon: false,
        config_entries,
    };
    Ok(server_settings)
}

pub fn generate_command_line(
    config_metadata: &ConfigMetadataState,
    server_settings: &ServerSettings,
) -> Result<Vec<String>> {
    let mut args: Vec<String> = Vec::new();

    let config_metadata = config_metadata.effective();
    // Map metadata to each entry
    let settings_meta_map = server_settings
        .config_entries
        .entries
        .iter()
        .filter(|e| e.meta_name != "additionalOptions") // We handle this separately below
        .filter_map(|e| {
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
        .collect::<Vec<_>>();

    if settings_meta_map.len() < server_settings.config_entries.entries.len() {
        for entry in server_settings.config_entries.entries.iter() {
            if !settings_meta_map.iter().any(|(c, _)| {
                c.meta_name == entry.meta_name && c.meta_location == entry.meta_location
            }) {
                warn!(
                    "Failed to find metadata for entry {} [{}]. Setting not applied to INI files",
                    entry.meta_name, entry.meta_location
                );
            }
        }
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

    let additional_options = server_settings
        .config_entries
        .entries
        .iter()
        .find(|e| e.meta_name == "additionalOptions");

    let mut url_params = settings_meta_map
        .iter()
        .filter(|(e, _)| e.meta_location == ConfigLocation::MapUrlOption)
        .map(|(e, _)| format!("{}={}", e.meta_name, e.value))
        .collect::<Vec<_>>()
        .join("?");

    if let Some(additional_options) = additional_options {
        if let ConfigVariant::Vector(values) = &additional_options.value {
            url_params += values
                .iter()
                .filter_map(|v| {
                    if let ConfigValue::String(s) = v {
                        if s.starts_with('?') {
                            Some(s.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("")
                .as_str();
        }
    }

    let mut switch_params = settings_meta_map
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
                format!("-{}={}", e.meta_name, e.value)
            }
        })
        .collect::<Vec<_>>();

    if let Some(additional_options) = additional_options {
        if let ConfigVariant::Vector(values) = &additional_options.value {
            switch_params.extend(values.iter().filter_map(|v| {
                if let ConfigValue::String(s) = v {
                    if s.starts_with('-') {
                        Some(s.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            }));
        }
    }

    if url_params.is_empty() {
        args.push(map.to_owned());
    } else {
        args.push(format!("{}?{}", map, url_params));
    }

    args.extend(switch_params);

    Ok(args)
}

/// Starts the server, returns the PID of the running process
pub async fn start_server(
    server_id: Uuid,
    server_name: impl AsRef<str>,
    installation_dir: impl AsRef<str>,
    use_server_api: bool,
    args: Vec<String>,
) -> Result<u32> {
    let installation_dir = installation_dir.as_ref();
    let exe_path = Path::new(installation_dir);
    // TODO: Refactor this out, it's shared with the monitor code
    let exe = if use_server_api {
        exe_path.join("ShooterGame/Binaries/Win64/AsaApiLoader.exe")
    } else {
        exe_path.join("ShooterGame/Binaries/Win64/ArkAscendedServer.exe")
    };

    let exe = exe.canonicalize().expect("Failed to canonicalize path");

    let _profile_descriptor = format!("\"ASA.{}.{}\"", server_id, server_name.as_ref());

    // If we want to tag the process with metadata, we either need to force set the title after launch,
    // or run it via a batch file using `start "<profile_descriptor>"` ...
    let mut command = Command::new(exe);
    command.args(args);
    command.kill_on_drop(false);
    #[cfg(windows)]
    {
        const DETACHED_PROCESS: u32 = 0x00000008;
        command.creation_flags(DETACHED_PROCESS);
    }

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
    Ok(pid)
}

#[cfg(all(windows, not(feature = "conpty")))]
pub mod os {
    use std::{path::Path, process::Stdio};

    use anyhow::{Context, Result};
    use regex::Regex;
    use tokio::{
        io::{AsyncBufReadExt, BufReader},
        process::{ChildStdout, Command},
        sync::mpsc::Sender,
    };
    use tracing::{error, trace, warn};
    use uuid::Uuid;

    use crate::{server::UpdateServerProgress, AsyncNotification};

    use super::UpdateMode;

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

        let steamcmd_exe = Path::new(&steamcmd_dir).join("steamcmd.exe");

        // Create the installation directory
        std::fs::create_dir_all(&installation_dir)
            .with_context(|| "Failed to create installation directory")?;

        let mut args = vec![
            "+force_install_dir",
            &installation_dir,
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

        trace!("SteamCMD: {} {}", steamcmd_exe.display(), args.join(" "));
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
                    process_steamcmd_line(server_id, line.trim(), &progress_parser, &progress)
                        .await;
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

    async fn process_steamcmd_line(
        server_id: Uuid,
        line: &str,
        progress_parser: &Regex,
        progress: &Sender<AsyncNotification>,
    ) {
        if let Some(captures) = progress_parser.captures(&line) {
            if captures.len() == 4 {
                let state = captures.name("state").expect("Failed to get state");
                let desc = captures.name("desc").expect("Failed to get desc");
                let percent = captures.name("percent").expect("Failed to get percent");

                let state =
                    u64::from_str_radix(state.as_str(), 16).expect("Failed to parse status code");
                let percent: f32 = percent.as_str().parse().expect("Failed to parse prpogress");

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
}

#[cfg(all(windows, feature = "conpty"))]
pub mod os {
    use std::{
        io::{ErrorKind, Read},
        path::{Path, PathBuf},
        time::Duration,
    };

    use anyhow::{Context, Result};
    use regex::Regex;
    use tokio::sync::mpsc::Sender;
    use tracing::{trace, warn};
    use uuid::Uuid;

    use crate::{server::UpdateServerProgress, AsyncNotification};

    use super::UpdateMode;

    pub async fn update_server(
        server_id: Uuid,
        steamcmd_dir: impl AsRef<str>,
        installation_dir: impl AsRef<str>,
        app_id: impl AsRef<str>,
        mode: UpdateMode,
        progress: Sender<AsyncNotification>,
    ) -> Result<()> {
        let steamcmd_dir = steamcmd_dir.as_ref().to_owned();
        let installation_dir = installation_dir.as_ref().to_owned();
        let app_id = app_id.as_ref().to_owned();
        let handle = tokio::task::spawn_blocking(move || {
            update_server_thread(
                server_id,
                steamcmd_dir,
                installation_dir,
                app_id,
                mode,
                progress,
            )
        });
        handle.await?
    }

    fn update_server_thread(
        server_id: Uuid,
        steamcmd_dir: String,
        installation_dir: String,
        app_id: String,
        mode: UpdateMode,
        progress: Sender<AsyncNotification>,
    ) -> Result<()> {
        let steamcmd_exe = Path::new(&steamcmd_dir).join("steamcmd.exe");

        // Create the installation directory
        std::fs::create_dir_all(&installation_dir)
            .with_context(|| "Failed to create installation directory")?;

        let installation_dir_arg = &format!(r#""{}""#, &installation_dir);
        let mut args = vec![
            "+force_install_dir",
            &installation_dir_arg,
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

        run_steamcmd_conpty(server_id, steamcmd_exe, &args, progress)
    }

    fn run_steamcmd_conpty(
        server_id: Uuid,
        steamcmd_exe: PathBuf,
        args: &[&str],
        progress: Sender<AsyncNotification>,
    ) -> Result<()> {
        trace!("SteamCMD: {} {}", steamcmd_exe.display(), args.join(" "));

        // This is due to the fact that conpty runs the command under `cmd.exe` which has weird quoting
        // rules when there are possibly multiple sets of quote on the line.  This allow us to have spaces
        // in the SteamCMD path, as well as spaces in the installation path.
        let steamcmd_string = steamcmd_exe.to_str().to_owned().unwrap().replace(' ', "^ ");
        let command_line = format!(r#"{} {}"#, steamcmd_string, args.join(" "));

        trace!("Running SteamCmd: {}", command_line);
        let progress_parser = Regex::new(
            r"Update state \(0x(?<state>[0-9a-fA-F]+)\) (?<desc>[^,]*), progress: (?<percent>[0-9.]+)",
        )
        .expect("Failed to compile progress regex");

        let _ = progress.blocking_send(AsyncNotification::UpdateServerProgress(
            server_id,
            UpdateServerProgress::Initializing,
        ));

        let mut process = conpty::spawn(&command_line)
            .unwrap_or_else(|_| panic!("Failed to spawn {}", command_line));

        let mut output = process.output().expect("Failed to get output pipe");
        output.blocking(false);

        trace!("SteamCMD: Starting read");
        let mut buf = vec![0u8; 64];
        let mut line_buf = String::new();
        loop {
            match output.read(&mut buf) {
                Ok(bytes_read) => {
                    if bytes_read > 0 {
                        let buf_as_str = std::str::from_utf8(&buf[0..bytes_read]).unwrap();
                        if let Some(index) = buf_as_str.find('\r') {
                            // Push the rest of this line
                            line_buf.push_str(&buf_as_str[0..index]);
                            process_steamcmd_line(
                                server_id,
                                line_buf.trim(),
                                &progress_parser,
                                &progress,
                            );
                            // Start a new line
                            line_buf.clear();
                            line_buf.push_str(&buf_as_str[index..]);
                        } else {
                            // Add to the current line
                            line_buf.push_str(buf_as_str);
                        }
                    } else if !process.is_alive() {
                        trace!("Process exited.");
                        break;
                    } else {
                        trace!("Waiting...");
                        std::thread::sleep(Duration::from_millis(500));
                    }
                }
                Err(e) => {
                    if e.kind() == ErrorKind::WouldBlock {
                        if !process.is_alive() {
                            trace!("Process exited while waiting");
                            break;
                        } else {
                            std::thread::sleep(Duration::from_millis(500));
                        }
                    } else {
                        trace!("Error reading from pipe: {:?}", e);
                        break;
                    }
                }
            }
        }

        trace!("Update finished");
        Ok(())
    }

    fn process_steamcmd_line(
        server_id: Uuid,
        line: &str,
        progress_parser: &Regex,
        progress: &Sender<AsyncNotification>,
    ) {
        if let Some(captures) = progress_parser.captures(line) {
            if captures.len() == 4 {
                let state = captures.name("state").expect("Failed to get state");
                let desc = captures.name("desc").expect("Failed to get desc");
                let percent = captures.name("percent").expect("Failed to get percent");

                let state =
                    u64::from_str_radix(state.as_str(), 16).expect("Failed to parse status code");
                let percent: f32 = percent.as_str().parse().expect("Failed to parse prpogress");

                match state {
                    0x61 => {
                        trace!("{}: SteamCMD: Downloading {}", server_id, percent);
                        let _ = progress.blocking_send(AsyncNotification::UpdateServerProgress(
                            server_id,
                            UpdateServerProgress::Downloading(percent),
                        ));
                    }
                    0x81 => {
                        trace!("{}: SteamCMD: Verifying {}", server_id, percent);
                        let _ = progress.blocking_send(AsyncNotification::UpdateServerProgress(
                            server_id,
                            UpdateServerProgress::Verifying(percent),
                        ));
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
}

// NOTE: PERFORMANCE: This algorithm works reasonably, but can take several seconds on debug builds.
async fn get_asa_version(exe_path: &PathBuf) -> Result<String> {
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
    let mut last_yield_time = Instant::now();
    let mut bytes_read_since_last_yield = 0usize;
    loop {
        bytes_read.clear();
        if read_to_byte(&mut reader, target_bytes[0]) {
            bytes_read_since_last_yield += 1;

            if bytes_read_since_last_yield > 100000 {
                let now = Instant::now();
                if Instant::now() - last_yield_time > Duration::from_millis(100) {
                    yield_now().await;
                    last_yield_time = now;
                    bytes_read_since_last_yield = 0;
                }
            }
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
    Success {
        version: String,
        install_time: DateTime<Local>,
        build_id: u64,
        time_updated: u64,
        server_api_state: ServerApiState,
    },
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

    let (time_updated, build_id) = match std::fs::read_to_string(manifest_path) {
        Err(err) => match err.kind() {
            ErrorKind::NotFound => {
                trace!("{}: No appmanifest found", id);
                return Ok(ValidationResult::NotInstalled);
            }
            _ => return Err(err.into()),
        },
        Ok(content) => {
            let state = extract_app_state_field(&content, "StateFlags")
                .and_then(|v| v.parse::<u32>().ok())
                .with_context(|| "Failed to find or parse StateFlags")?;
            if state != STATE_INSTALL_SUCCESSFUL {
                trace!("{}: Incomplete install (state = {})", id, state);
                return Ok(ValidationResult::Failed("Incomplete".to_string()));
            }

            let time_updated = extract_app_state_field(&content, "LastUpdated")
                .and_then(|v| v.parse().ok())
                .with_context(|| "Failed to find or parse LastUpdated")?;

            let build_id = extract_app_state_field(&content, "buildid")
                .and_then(|v| v.parse().ok())
                .with_context(|| "buildid failed to parse")?;
            (time_updated, build_id)
        }
    };

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
    let version = get_asa_version(&binary_path).await?;

    let install_time: DateTime<Local> =
        DateTime::from(metadata.created().with_context(|| "No Creation Time")?);

    // See if ServerApi is installed
    let server_api_state = check_server_api_install_state(installation_dir);

    Ok(ValidationResult::Success {
        version,
        install_time,
        time_updated,
        build_id,
        server_api_state,
    })
}

fn make_field_regex(field: &str) -> Regex {
    let regex = format!(r#"{}\"[^"]+\"(?<value>[^"]*)"#, field);
    Regex::new(&regex).expect("Failed to build manifest searching regex")
}

fn extract_app_state_field<'a>(content: &'a str, field: &str) -> Option<&'a str> {
    let regex = make_field_regex(field);
    regex
        .captures(content)
        .and_then(|c| c.name("value"))
        .map(|m| m.as_str())
}
