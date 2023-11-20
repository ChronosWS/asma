use std::{collections::HashMap, path::{PathBuf, Path}, time::Duration};

use sysinfo::{System, Pid, SystemExt, ProcessExt, ProcessStatus, PidExt};
use tokio::{sync::mpsc::{Receiver, Sender, error::TryRecvError}, time::sleep};
use anyhow::Result;
use tracing::{trace, warn, error};
use uuid::Uuid;

use crate::{AsyncNotification, models::{RunState, RunData}};

pub enum ServerMonitorCommand {
    AddServer { server_id: Uuid, installation_dir: String },
    KillServer { server_id: Uuid }
}

struct ServerProcessRecord {
    server_id: Uuid,
    exe_path: PathBuf,
    pid: Pid
}

/// Watches the process stack for changes to this server's process state
pub async fn monitor_server(
    mut command: Receiver<ServerMonitorCommand>,
    progress: Sender<AsyncNotification>,
) -> Result<()> {
    let mut system = System::default();
    let mut server_processes = HashMap::new();
    let mut dead_servers = Vec::new();
    
    loop {
        // Check for new commands
        let command = command.try_recv();
        match command {
            Ok(ServerMonitorCommand::AddServer { server_id, installation_dir }) => {
                let path = Path::new(&installation_dir)
                .join("ShooterGame/Binaries/Win64/ArkAscendedServer.exe");
                if let Ok(exe_path) = path.canonicalize() {
                    trace!("Initializing server monitoring for {} ({})", server_id, exe_path.display());
                    // Refresh all processes so we can find the PID in the set of command-lines
                    system.refresh_processes();
                    let process = system.processes().values().find(|process| 
                        process
                            .exe()
                            .canonicalize()
                            .map(|process_exe| process_exe == exe_path)
                            .unwrap_or(false));
                    if let Some(process) = process {
                        let pid = process.pid();
                        server_processes.insert(server_id, ServerProcessRecord {
                            server_id,
                            exe_path,
                            pid
                        });
                    } else {
                        warn!("Failed to find server process for {} ({}).  This might be OK on startup if the server isn't running", server_id, exe_path.display());
                    }
                } else {
                    error!("Failed to canonicalize path {}", path.display())
                }
            },
            Ok(ServerMonitorCommand::KillServer { server_id }) => {
                if let Some(record) = server_processes.get(&server_id) {
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
                // Do nothing
            }
        }

        // Refresh the process state for each tracked process
        for record in server_processes.values() {
            let process_exists = system.refresh_process(record.pid);
            if !process_exists {
                // The process has terminated
                let _ = progress
                            .send(AsyncNotification::UpdateServerRunState(
                                record.server_id,
                                RunState::Stopped,
                            ))
                            .await;
                dead_servers.push(record.server_id);
            } else if let Some(process) = system.process(record.pid) {
                match process.status() {
                    ProcessStatus::Run => {
                        let run_data = RunData {
                            pid: record.pid.as_u32(),
                            cpu_usage: process.cpu_usage(),
                            memory_usage: process.memory(),
                        };
                        let _ = progress
                            .send(AsyncNotification::UpdateServerRunState(
                                record.server_id,
                                RunState::Available(run_data),
                            ))
                            .await;
                    }
                    other => {
                        trace!("{}: Other Status: {:?}.  Bailing...", record.server_id, other);
                        break;
                    }
                }
            } else {
                // Somehow didn't find the process
                error!("Failed to fine process {} ({})", record.server_id, record.exe_path.display());
                dead_servers.push(record.server_id);
            }
        }

        // Remove records of dead servers
        dead_servers.drain(..).for_each(|server_id| { server_processes.remove(&server_id); });

        sleep(Duration::from_secs(5)).await;
    }
}
