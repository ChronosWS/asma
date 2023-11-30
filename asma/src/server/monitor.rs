use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result};
use rcon::Connection;
use regex::Regex;
use reqwest::Url;
use sysinfo::{Pid, PidExt, ProcessExt, ProcessStatus, System, SystemExt};
use tokio::{
    sync::mpsc::{channel, error::TryRecvError, Receiver, Sender},
    task::JoinSet,
    time::{sleep, timeout, Instant},
};
use tracing::{error, trace, warn};
use uuid::Uuid;

use crate::{
    models::{RunData, RunState},
    update_utils::{check_for_asma_updates, update_asma, AsmaUpdateState},
    AsyncNotification, steamapi_utils::check_for_steam_updates,
};

pub struct RconMonitorSettings {
    pub address: String,
    pub password: String,
}
pub enum ServerMonitorCommand {
    AddServer {
        server_id: Uuid,
        installation_dir: String,
        rcon_settings: Option<RconMonitorSettings>,
    },
    StopServer {
        server_id: Uuid,
    },
    KillServer {
        server_id: Uuid,
    },
    UpdateAsma,
    CheckForAsmaUpdates,
    CheckForServerUpdates,
}

#[derive(Debug, Clone)]
pub struct RconExecResponse {
    id: i32,
    response: String,
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct RconPlayerEntry {
    player_num: usize,
    steam_id: String,
    user_name: String,
}

#[allow(unused)]
enum RconCommand {
    Stop,
    Exec { id: i32, command: String },
}

#[derive(Debug, Clone)]
pub enum RconResponse {
    Stopped,
    Connected,
    ExecResponse(RconExecResponse),
}

enum RconState {
    NotConnected {
        command_sender: Sender<RconCommand>,
        response_receiver: Receiver<RconResponse>,
    },
    Connected {
        command_sender: Sender<RconCommand>,
        response_receiver: Receiver<RconResponse>,
    },
}

struct ServerProcessRecord {
    server_id: Uuid,
    exe_path: PathBuf,
    pid: Pid,
    rcon_state: Option<RconState>,
    is_stopping: bool,
}

pub struct MonitorConfig {
    pub app_update_url: Url,
    pub app_update_check_seconds: u64,
    pub steam_api_key: String,
    pub steam_app_id: String,
    pub server_update_check_seconds: u64,
}

// Special RCON queries that don't bubble up
const EXEC_LIST_PLAYERS: i32 = -1;
const EXEC_LIST_PLAYERS_COMMAND: &str = "ListPlayers";

const EXEC_STOP: i32 = -2;
const EXEC_STOP_COMMAND: &str = "DoExit";

/// Watches the process stack for changes to this server's process state
pub async fn monitor_server(
    monitor_config: MonitorConfig,
    mut command: Receiver<ServerMonitorCommand>,
    status_sender: Sender<AsyncNotification>,
) -> Result<()> {
    let mut system = System::default();
    let mut server_records = HashMap::new();
    let mut dead_servers = Vec::new();
    let mut rcon_runner_tasks: JoinSet<Result<()>> = JoinSet::new();
    let mut rcon_responses = Vec::new();
    let mut player_list = Vec::<RconPlayerEntry>::new();
    let mut last_asma_update_check = None;
    let mut last_server_update_check = None;
    let player_list_regex = Regex::new("(?<num>[0-9]+). (?<name>[^,]+), (?<userid>[0-9a-f]+)")
        .expect("Failed to compile player list regex");
    loop {
        loop {
            // Check for new commands
            let command = timeout(Duration::from_secs(5), command.recv()).await;
            match command {
                Ok(Some(ServerMonitorCommand::AddServer {
                    server_id,
                    installation_dir,
                    rcon_settings,
                })) => {
                    let path = Path::new(&installation_dir)
                        .join("ShooterGame/Binaries/Win64/ArkAscendedServer.exe");
                    if std::fs::metadata(&path).is_ok() {
                        if let Ok(exe_path) = path.canonicalize() {
                            trace!(
                                "Initializing server monitoring for {} ({})",
                                server_id,
                                exe_path.display()
                            );
                            // Refresh all processes so we can find the PID in the set of command-lines
                            system.refresh_processes();
                            let process = system.processes().values().find(|process| {
                                process
                                    .exe()
                                    .canonicalize()
                                    .map(|process_exe| process_exe == exe_path)
                                    .unwrap_or(false)
                            });
                            if let Some(process) = process {
                                let pid = process.pid();

                                let rcon_state = if let Some(rcon_settings) = rcon_settings {
                                    let (command_send, command_recv) = channel(100);
                                    let (response_send, response_recv) = channel(100);
                                    // TODO:
                                    rcon_runner_tasks.spawn(rcon_runner(
                                        server_id.to_owned(),
                                        rcon_settings,
                                        command_recv,
                                        response_send,
                                    ));
                                    Some(RconState::NotConnected {
                                        command_sender: command_send,
                                        response_receiver: response_recv,
                                    })
                                } else {
                                    None
                                };

                                server_records.insert(
                                    server_id,
                                    ServerProcessRecord {
                                        server_id,
                                        exe_path,
                                        pid,
                                        rcon_state,
                                        is_stopping: false,
                                    },
                                );
                            } else {
                                warn!("Failed to find server process for {} ({}).  This might be OK on startup if the server isn't running", server_id, exe_path.display());
                                // TODO: These failure path calls could use some cleanup
                                let _ = status_sender
                                    .send(AsyncNotification::UpdateServerRunState(
                                        server_id,
                                        RunState::Stopped,
                                    ))
                                    .await;
                            }
                        } else {
                            error!("Failed to canonicalize path {}", path.display());
                            let _ = status_sender
                                .send(AsyncNotification::UpdateServerRunState(
                                    server_id,
                                    RunState::Stopped,
                                ))
                                .await;
                        }
                    } else {
                        warn!(
                            "Path {} doesn't exist - maybe this server isn't installed yet?",
                            path.display()
                        );
                        let _ = status_sender
                            .send(AsyncNotification::UpdateServerRunState(
                                server_id,
                                RunState::Stopped,
                            ))
                            .await;
                    }
                }
                Ok(Some(ServerMonitorCommand::StopServer { server_id })) => {
                    if let Some(record) = server_records.get_mut(&server_id) {
                        try_send_rcon_command(
                            record.server_id,
                            &record.rcon_state,
                            EXEC_STOP,
                            EXEC_STOP_COMMAND,
                        )
                        .await;
                        record.is_stopping = true;
                    }
                }
                Ok(Some(ServerMonitorCommand::KillServer { server_id })) => {
                    if let Some(record) = server_records.get_mut(&server_id) {
                        if let Some(process) = system.process(record.pid) {
                            trace!("Sending KILL to {}", record.pid);
                            process.kill_with(sysinfo::Signal::Kill);
                            record.is_stopping = true;
                        }
                    }
                }
                Ok(Some(ServerMonitorCommand::UpdateAsma)) => {
                    match update_asma(&status_sender, &monitor_config.app_update_url).await {
                        Ok(_) => {
                            let _ = status_sender
                                .send(AsyncNotification::AsmaUpdateState(
                                    AsmaUpdateState::UpdateReady,
                                ))
                                .await;
                        }
                        Err(e) => {
                            warn!("ASMA update failed: {}", e.to_string());
                            let _ = status_sender
                                .send(AsyncNotification::AsmaUpdateState(
                                    AsmaUpdateState::UpdateFailed,
                                ))
                                .await;
                        }
                    }
                }
                Ok(Some(ServerMonitorCommand::CheckForAsmaUpdates)) => {
                    last_asma_update_check = None
                }
                Ok(Some(ServerMonitorCommand::CheckForServerUpdates)) => {
                    last_server_update_check = None
                }
                Ok(None) => {
                    trace!("Closing monitor_server channel");
                    return Ok(());
                }
                Err(_elapsed) => {
                    // Timed out waiting for commands
                    break;
                }
            }
        }

        // Check for ASMA updates
        if let Some(last_checked_time) = last_asma_update_check {
            let now = Instant::now();
            if now - last_checked_time
                > Duration::from_secs(monitor_config.app_update_check_seconds)
            {
                let _ = check_for_asma_updates(&status_sender, &monitor_config.app_update_url)
                    .await
                    .map_err(|e| {
                        warn!("Failed to get latest ASMA version info: {}", e.to_string())
                    });
                last_asma_update_check = Some(now)
            }
        } else {
            // First boot check
            let _ = check_for_asma_updates(&status_sender, &monitor_config.app_update_url)
                .await
                .map_err(|e| warn!("Failed to get latest ASMA version info: {}", e.to_string()));
            last_asma_update_check = Some(Instant::now())
        }

        // Check for server updates
        if let Some(last_checked_time) = last_server_update_check {
            let now = Instant::now();
            if now - last_checked_time
                > Duration::from_secs(monitor_config.server_update_check_seconds)
            {
                let _ = check_for_steam_updates(
                    &status_sender,
                    &monitor_config.steam_app_id,
                )
                .await
                .map_err(|e| {
                    warn!(
                        "Failed to get latest server version info: {}",
                        e.to_string()
                    )
                });
                last_server_update_check = Some(now)
            }
        } else {
            // First boot check
            let _ = check_for_steam_updates(
                &status_sender,
                &monitor_config.steam_app_id,
            )
            .await
            .map_err(|e| {
                warn!(
                    "Failed to get latest server version info: {}",
                    e.to_string()
                )
            });
            last_server_update_check = Some(Instant::now())
        }

        // Check the status of each server now
        for record in server_records.values_mut() {
            rcon_responses.clear();
            record.rcon_state = rcon_pump(
                record.server_id,
                record.rcon_state.take(),
                &mut rcon_responses,
            )
            .await;
            player_list.clear();
            if let Some(list_players_response) = rcon_responses
                .iter()
                .rev()
                .find(|r| r.id == EXEC_LIST_PLAYERS)
            {
                for (_, [num, name, user_id]) in player_list_regex
                    .captures_iter(&list_players_response.response)
                    .map(|c| c.extract())
                {
                    if let Ok(player_num) = num.parse::<usize>().map_err(|e| {
                        error!("Failed to parse player number {}: {}", num, e.to_string())
                    }) {
                        player_list.push(RconPlayerEntry {
                            player_num,
                            steam_id: user_id.to_owned(),
                            user_name: name.to_owned(),
                        })
                    }
                }
            }

            try_send_rcon_command(
                record.server_id,
                &record.rcon_state,
                EXEC_LIST_PLAYERS,
                EXEC_LIST_PLAYERS_COMMAND,
            )
            .await;
            let rcon_enabled = if let Some(RconState::Connected { .. }) = &record.rcon_state {
                true
            } else {
                false
            };

            let process_exists = system.refresh_process(record.pid);
            if !process_exists {
                // The process has terminated
                let _ = status_sender
                    .send(AsyncNotification::UpdateServerRunState(
                        record.server_id,
                        RunState::Stopped,
                    ))
                    .await;
                dead_servers.push(record.server_id);
            } else if let Some(process) = system.process(record.pid) {
                match process.status() {
                    ProcessStatus::Run => {
                        // TODO: How do we want to handle asking for players?  From the runner?

                        let run_data = RunData {
                            pid: record.pid.as_u32(),
                            cpu_usage: process.cpu_usage(),
                            memory_usage: process.memory(),
                            rcon_enabled,
                            player_list: player_list.clone(),
                        };
                        let _ = status_sender
                            .send(AsyncNotification::UpdateServerRunState(
                                record.server_id,
                                if record.is_stopping {
                                    RunState::Stopping
                                } else {
                                    RunState::Available(run_data)
                                },
                            ))
                            .await;
                    }
                    other => {
                        trace!(
                            "{}: Other Status: {:?}.  Bailing...",
                            record.server_id,
                            other
                        );
                        break;
                    }
                }
            } else {
                // Somehow didn't find the process
                error!(
                    "Failed to fine process {} ({})",
                    record.server_id,
                    record.exe_path.display()
                );
                dead_servers.push(record.server_id);
            }
        }

        // Remove records of dead servers
        dead_servers.drain(..).for_each(|server_id| {
            trace!("Monitor: Removing dead server {}", server_id);
            server_records.remove(&server_id);
        });

        // trace!("Monitor: Sleeping...");
        sleep(Duration::from_secs(5)).await;
    }
}

async fn try_send_rcon_command(
    server_id: Uuid,
    rcon_state: &Option<RconState>,
    id: i32,
    command: impl ToString,
) {
    if let Some(RconState::Connected { command_sender, .. }) = rcon_state {
        if let Err(e) = command_sender.try_send(RconCommand::Exec {
            id,
            command: command.to_string(),
        }) {
            warn!("Monitor {}: Error sending command: {:?}", server_id, e);
        } else {
            // trace!(
            //     "Monitor {}: Sent command: {}",
            //     record.server_id,
            //     "ListPlayers"
            // );
        }
    }
}
async fn rcon_pump(
    server_id: Uuid,
    rcon_state: Option<RconState>,
    rcon_responses: &mut Vec<RconExecResponse>,
) -> Option<RconState> {
    match rcon_state {
        Some(RconState::NotConnected {
            command_sender,
            mut response_receiver,
        }) => {
            trace!("Monitor {}: NotConnected state", server_id);
            match response_receiver.try_recv() {
                Ok(RconResponse::Connected) => {
                    trace!("Monitor {}: RCON connected", server_id);
                    Some(RconState::Connected {
                        command_sender,
                        response_receiver,
                    })
                }
                Err(TryRecvError::Empty) => {
                    // Nothing to read yet
                    trace!("Monitor {}: Nothing to read yet", server_id);
                    Some(RconState::NotConnected {
                        command_sender,
                        response_receiver,
                    })
                }
                Err(TryRecvError::Disconnected) => {
                    // TODO: Kill rcon task?
                    warn!("Monitor {}: RCON disconnected", server_id);
                    Some(RconState::NotConnected {
                        command_sender,
                        response_receiver,
                    })
                }
                _ => {
                    warn!(
                        "Monitor {}: Unexpected RCON response while disconnected",
                        server_id
                    );
                    Some(RconState::NotConnected {
                        command_sender,
                        response_receiver,
                    })
                }
            }
        }
        Some(RconState::Connected {
            command_sender,
            mut response_receiver,
        }) => {
            // trace!("Monitor {}: Performing RCON pump", server_id);
            // Check for responses
            match response_receiver.try_recv() {
                Ok(RconResponse::ExecResponse(response)) => {
                    // trace!(
                    //     "Monitor {}: RCON Response: ({}) {}",
                    //     server_id,
                    //     response.id,
                    //     response.response
                    // );
                    rcon_responses.push(response);
                    Some(RconState::Connected {
                        command_sender,
                        response_receiver,
                    })
                }
                Ok(RconResponse::Stopped) => {
                    trace!("Monitor {}: RCON Stopped", server_id);
                    None
                }
                Err(TryRecvError::Empty) => {
                    // Do nothing
                    Some(RconState::Connected {
                        command_sender,
                        response_receiver,
                    })
                }
                Err(TryRecvError::Disconnected) => {
                    // TODO: Kill rcon task?
                    warn!("Monitor {}: RCON disconnected", server_id);
                    None
                }
                r => {
                    warn!("Monitor {}: Unexpected response: {:?}", server_id, r);
                    Some(RconState::Connected {
                        command_sender,
                        response_receiver,
                    })
                }
            }
        }
        None => None,
    }
}

async fn rcon_runner(
    server_id: Uuid,
    rcon_settings: RconMonitorSettings,
    mut command_receiver: Receiver<RconCommand>,
    response_sender: Sender<RconResponse>,
) -> Result<()> {
    let mut connection: Option<Connection> = None;
    loop {
        if let Some(connection) = &mut connection {
            if let Some(rcon_command) = command_receiver.recv().await {
                match rcon_command {
                    RconCommand::Stop => {
                        trace!("RCON {} ({}): Stopping", server_id, rcon_settings.address);
                        return Ok(());
                    }
                    RconCommand::Exec { id, command } => {
                        let response = connection
                            .cmd(&command)
                            .await
                            .with_context(|| {
                                format!("RCON [{}] '{}' failed", rcon_settings.address, command)
                            })
                            .map(|(_, r)| r)
                            .with_context(|| "Error sending command")?;
                        trace!(
                            "RCON {} ({}): Command ({}): {} Response: {}",
                            server_id,
                            rcon_settings.address,
                            id,
                            command,
                            response.trim_end()
                        );
                        match response_sender
                            .send(RconResponse::ExecResponse(RconExecResponse {
                                id,
                                response,
                            }))
                            .await
                        {
                            Ok(()) => {
                                // Do nothing
                            }
                            Err(e) => {
                                error!(
                                    "RCON {} ({}): Failed to send response: {}",
                                    server_id,
                                    rcon_settings.address,
                                    e.to_string()
                                );
                            }
                        }
                    }
                }
            }
        } else {
            // Discard all pending commands
            loop {
                match command_receiver.try_recv() {
                    Ok(_) => {}
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => return Ok(()),
                }
            }

            match timeout(
                Duration::from_millis(5000),
                Connection::connect(&rcon_settings.address, &rcon_settings.password),
            )
            .await
            {
                Ok(Ok(result)) => {
                    trace!("RCON {} ({}): Connected", server_id, rcon_settings.address);
                    connection = Some(result);
                    response_sender
                        .send(RconResponse::Connected)
                        .await
                        .with_context(|| "Failed to send Connected response")?;
                }
                Ok(_) => {
                    warn!(
                        "RCON {} ({}): Failed to connect",
                        server_id, rcon_settings.address
                    );
                }
                Err(_) => {
                    warn!(
                        "RCON {} ({}): Timed out trying to connect",
                        server_id, rcon_settings.address
                    )
                }
            }
        }
    }
}
