use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result};
use rcon::Connection;
use regex::Regex;
use sysinfo::{Pid, PidExt, ProcessExt, ProcessStatus, System, SystemExt};
use tokio::{
    sync::mpsc::{channel, error::TryRecvError, Receiver, Sender},
    task::JoinSet,
    time::{sleep, timeout},
};
use tracing::{error, trace, warn};
use uuid::Uuid;

use crate::{
    models::{RunData, RunState},
    AsyncNotification,
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
}

#[derive(Debug, Clone)]
pub struct RconExecResponse {
    id: i32,
    response: String,
}

#[derive(Debug, Clone)]
pub struct RconPlayerEntry {
    player_num: usize,
    steam_id: String,
    user_name: String,
}

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
}

// Special RCON queries that don't bubble up
const LIST_PLAYERS_QUERY: i32 = -1;

/// Watches the process stack for changes to this server's process state
pub async fn monitor_server(
    mut command: Receiver<ServerMonitorCommand>,
    status_sender: Sender<AsyncNotification>,
) -> Result<()> {
    let mut system = System::default();
    let mut server_records = HashMap::new();
    let mut dead_servers = Vec::new();
    let mut rcon_runner_tasks: JoinSet<Result<()>> = JoinSet::new();
    let mut rcon_responses = Vec::new();
    let mut player_list = Vec::<RconPlayerEntry>::new();
    let player_list_regex = Regex::new("(?<num>[0-9]+). (?<name>[^,]+), (?<userid>[0-9a-f]+)")
        .expect("Failed to compile player list regex");
    loop {
        loop {
            // Check for new commands
            let command = command.try_recv();
            match command {
                Ok(ServerMonitorCommand::AddServer {
                    server_id,
                    installation_dir,
                    rcon_settings,
                }) => {
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
                                    },
                                );
                            } else {
                                warn!("Failed to find server process for {} ({}).  This might be OK on startup if the server isn't running", server_id, exe_path.display());
                            }
                        } else {
                            error!("Failed to canonicalize path {}", path.display())
                        }
                    } else {
                        warn!(
                            "Path {} doesn't exist - maybe this server isn't installed yet?",
                            path.display()
                        );
                    }
                }
                Ok(ServerMonitorCommand::StopServer { server_id }) => {
                    if let Some(_record) = server_records.get(&server_id) {
                        todo!("Stop the server nicely, if RCON is set up");
                    }
                }
                Ok(ServerMonitorCommand::KillServer { server_id }) => {
                    if let Some(record) = server_records.get(&server_id) {
                        if let Some(process) = system.process(record.pid) {
                            trace!("Sending KILL to {}", record.pid);
                            process.kill_with(sysinfo::Signal::Kill);
                        }
                    }
                }
                Err(TryRecvError::Disconnected) => {
                    trace!("Closing monitor_server channel");
                    return Ok(());
                }
                Err(TryRecvError::Empty) => {
                    // No more commands
                    break;
                }
            }
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
                .find(|r| r.id == LIST_PLAYERS_QUERY)
            {
                for (_, [num, name, user_id]) in player_list_regex
                    .captures_iter(&list_players_response.response)
                    .map(|c| c.extract())
                {
                    if let Ok(player_num) = num
                        .parse::<usize>()
                        .map_err(|e| error!("Failed to parse player number {}: {}", num, e.to_string()))
                    {
                        player_list.push(RconPlayerEntry {
                            player_num,
                            steam_id: user_id.to_owned(),
                            user_name: name.to_owned(),
                        })
                    }
                }
            }

            if let Some(RconState::Connected { command_sender, .. }) = &mut record.rcon_state {
                if let Err(e) = command_sender.try_send(RconCommand::Exec {
                    id: LIST_PLAYERS_QUERY,
                    command: "ListPlayers".to_owned(),
                }) {
                    warn!(
                        "Monitor {}: Error sending command: {:?}",
                        record.server_id, e
                    );
                } else {
                    trace!(
                        "Monitor {}: Sent command: {}",
                        record.server_id,
                        "ListPlayers"
                    );
                }
            }

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
                            player_list: player_list.clone()
                        };
                        let _ = status_sender
                            .send(AsyncNotification::UpdateServerRunState(
                                record.server_id,
                                RunState::Available(run_data),
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

        trace!("Monitor: Sleeping...");
        sleep(Duration::from_secs(5)).await;
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
            trace!("Monitor {}: Peforming RCON pump", server_id);
            // Check for responses
            match response_receiver.try_recv() {
                Ok(RconResponse::ExecResponse(response)) => {
                    trace!(
                        "Monitor {}: RCON Response: ({}) {}",
                        server_id,
                        response.id,
                        response.response
                    );
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
                        trace!(
                            "RCON {} ({}): Executing command: ({}) {}",
                            server_id,
                            rcon_settings.address,
                            id,
                            command
                        );
                        let response = connection
                            .cmd(&command)
                            .await
                            .with_context(|| {
                                format!("RCON [{}] '{}' failed", rcon_settings.address, command)
                            })
                            .map(|(_, r)| r)
                            .with_context(|| "Error sending command")?;
                        trace!(
                            "RCON {} ({}): Response: ({}) {}",
                            server_id,
                            rcon_settings.address,
                            id,
                            response
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
