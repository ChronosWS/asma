
#[cfg(all(windows, not(feature = "conpty")))]
pub use no_conpty::*;

#[cfg(all(windows, not(feature = "conpty")))]
pub mod no_conpty {
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

    use crate::{server::UpdateServerProgress, AsyncNotification, UpdateMode};

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
pub use conpty::*;

#[cfg(all(windows, feature = "conpty"))]
pub mod conpty {
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

    use crate::{server::UpdateServerProgress, AsyncNotification, UpdateMode};

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

