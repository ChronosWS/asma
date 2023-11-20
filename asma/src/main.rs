use std::fs::File;

use components::{make_button, server_card};
use config_utils::{create_metadata_index, rebuild_index_with_metadata, ConfigMetadataState};
use dialogs::global_settings::{self, GlobalSettingsMessage};
use dialogs::metadata_editor::{self, MetadataEditContext, MetadataEditorMessage};
use dialogs::server_settings::{self, ServerSettingsContext, ServerSettingsMessage};
use fonts::{get_system_font_bytes, BOLD_FONT};
use futures_util::SinkExt;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{column, container, horizontal_rule, row, scrollable, text};
use iced::{
    executor, font, subscription, Application, Color, Command, Element, Event, Length, Settings,
    Subscription, Theme,
};

use models::config::ConfigEntries;
use server::{ServerMonitorCommand, UpdateServerProgress, ValidationResult};
use steamcmd_utils::validate_steamcmd;
use sysinfo::{System, SystemExt};
use tantivy::Index;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::{channel, Sender};
use tracing::{error, trace, warn};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::{filter::LevelFilter, prelude::*, Layer};

mod components;
mod config_utils;
mod dialogs;
mod file_utils;
mod fonts;
mod icons;
mod modal;
mod models;
mod network_utils;
mod server;
mod settings_utils;
mod steamcmd_utils;

use modal::Modal;
use models::*;
use uuid::Uuid;

use crate::server::{monitor_server, start_server, update_server, validate_server, UpdateMode, update_inis_from_settings};

// iced uses a pattern based on the Elm architecture. To implement the pattern, the system is split
// into four parts:
// * The state
// * The messages, which communicate user interactions or events we care about
// * The view logic, which tells the system how to draw and maps user interactions to messages
// * The update logic, which processes messages and updates the state

enum MainWindowMode {
    Servers,
    GlobalSettings,
    EditProfile(ServerSettingsContext),
    MetadataEditor(MetadataEditContext),
}

struct AppState {
    monitor_command_channel: Option<Sender<ServerMonitorCommand>>,
    server_sender_channel: Option<Sender<AsyncNotification>>,
    global_settings: GlobalSettings,
    global_state: GlobalState,
    config_metadata_state: ConfigMetadataState,
    config_index: Index,
    servers: Vec<Server>,
    mode: MainWindowMode,
}

impl AppState {
    // TODO: These should probably just be changed to `get_server*` since settings
    // and state often go together and interior mutability is a PITA
    pub fn find_server(&self, id: Uuid) -> Option<(usize, &ServerSettings)> {
        self.servers
            .iter()
            .enumerate()
            .find(|(_, s)| s.settings.id == id)
            .map(|(i, s)| (i, &s.settings))
    }

    pub fn get_server_settings(&self, id: Uuid) -> Option<&ServerSettings> {
        self.servers
            .iter()
            .find(|s| s.settings.id == id)
            .map(|s| &s.settings)
    }

    pub fn get_server_state_mut(&mut self, id: Uuid) -> Option<&mut ServerState> {
        self.servers
            .iter_mut()
            .find(|s| s.settings.id == id)
            .map(|s| &mut s.state)
    }
}

#[derive(Debug, Clone)]
pub enum AsyncNotification {
    AsyncStarted(Sender<AsyncNotification>),
    UpdateServerProgress(Uuid, UpdateServerProgress),
    UpdateServerRunState(Uuid, RunState),
}

#[derive(Debug, Clone)]
pub enum Message {
    None,
    FontLoaded(Result<String, font::Error>),
    RefreshIp(LocalIp),

    // Dialogs
    GlobalSettings(GlobalSettingsMessage),
    ServerSettings(ServerSettingsMessage),
    MetadataEditor(MetadataEditorMessage),

    // Servers
    NewServer,
    EditServer(Uuid),
    InstallServer(Uuid, UpdateMode),
    ServerUpdated(Uuid),
    ServerValidated(Uuid, ValidationResult),
    StartServer(Uuid),
    StopServer(Uuid),
    ServerRunStateChanged(Uuid, RunState),

    // Keyboard and Mouse events
    Event(Event),

    // Notifications
    AsyncNotification(AsyncNotification),
}

impl From<GlobalSettingsMessage> for Message {
    fn from(value: GlobalSettingsMessage) -> Self {
        Message::GlobalSettings(value)
    }
}

impl From<ServerSettingsMessage> for Message {
    fn from(value: ServerSettingsMessage) -> Self {
        Message::ServerSettings(value)
    }
}

impl From<MetadataEditorMessage> for Message {
    fn from(value: MetadataEditorMessage) -> Self {
        Message::MetadataEditor(value)
    }
}

fn async_pump() -> Subscription<AsyncNotification> {
    struct Worker;
    subscription::channel(
        std::any::TypeId::of::<Worker>(),
        100,
        |mut output| async move {
            let (sender, mut receiver) = channel(100);
            let _ = output.send(AsyncNotification::AsyncStarted(sender)).await;
            loop {
                if let Some(message) = receiver.recv().await {
                    let _ = output.send(message).await;
                } else {
                    trace!("Async pump completed.");
                }
            }
        },
    )
}

async fn send_monitor_command(
    command_channel: Sender<ServerMonitorCommand>,
    command: ServerMonitorCommand,
) -> Result<(), SendError<ServerMonitorCommand>> {
    command_channel.send(command).await
}

impl Application for AppState {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        for signal in System::SUPPORTED_SIGNALS {
            trace!("Supported : {:?}", signal);
        }
        let arial_bytes = get_system_font_bytes("ARIAL.ttf").expect("Failed to find Arial");
        let global_settings = settings_utils::load_global_settings()
            .unwrap_or_else(|_| settings_utils::default_global_settings());
        let built_in_config_metadata = config_utils::load_built_in_config_metadata().unwrap();
        let local_config_metadata = config_utils::load_config_metadata().unwrap_or_default();
        let config_metadata_state = ConfigMetadataState::from_built_in_and_local(
            built_in_config_metadata,
            local_config_metadata,
        );

        let servers = settings_utils::load_server_settings(&global_settings)
            .expect("Failed to load server settings")
            .drain(..)
            .map(|settings| Server {
                settings,
                state: ServerState {
                    install_state: InstallState::Validating,
                    run_state: RunState::NotInstalled,
                },
            })
            .collect::<Vec<_>>();

        let mut startup_commands = vec![
            font::load(std::borrow::Cow::from(arial_bytes))
                .map(|v| Message::FontLoaded(v.map(|_| "Arial".into()))),
            Command::perform(network_utils::refresh_ip(), |result| {
                if let Ok(ip_addr) = result {
                    Message::RefreshIp(LocalIp::Resolved(ip_addr))
                } else {
                    Message::RefreshIp(LocalIp::Failed)
                }
            }),
        ];

        let mut validation_commands = servers
            .iter()
            .map(|s| {
                let id = s.id();
                let install_location = s.settings.get_full_installation_location();
                let app_id = global_settings.app_id.to_owned();
                Command::perform(
                    validate_server(id, install_location, app_id),
                    move |result| {
                        result
                            .map(|r| Message::ServerValidated(id, r))
                            .unwrap_or_else(|e| {
                                Message::ServerValidated(
                                    id,
                                    ValidationResult::Failed(e.to_string()),
                                )
                            })
                    },
                )
            })
            .collect();

        startup_commands.append(&mut validation_commands);

        let steamcmd_state = if validate_steamcmd(&global_settings.steamcmd_directory) {
            SteamCmdState::Installed
        } else {
            SteamCmdState::NotInstalled
        };

        let mut config_index = create_metadata_index();
        rebuild_index_with_metadata(
            &mut config_index,
            &config_metadata_state.effective().entries,
        )
        .expect("Failed to build config metadata index");

        (
            AppState {
                monitor_command_channel: None,
                server_sender_channel: None,
                global_settings,
                global_state: GlobalState {
                    app_version: env!("CARGO_PKG_VERSION").into(),
                    local_ip: LocalIp::Unknown,
                    edit_metadata_id: None,
                    steamcmd_state,
                },
                config_metadata_state,
                config_index,
                servers,
                mode: MainWindowMode::Servers,
            },
            Command::batch(startup_commands),
        )
    }

    fn title(&self) -> String {
        format!(
            "Ark Server Manager: Ascended (Version {})",
            self.global_state.app_version
        )
    }

    fn theme(&self) -> Theme {
        match self.global_settings.theme {
            ThemeType::Dark => Theme::Dark,
            ThemeType::Light => Theme::Light,
        }
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch([
            //subscription::events().map(Message::Event),
            async_pump().map(Message::AsyncNotification),
        ])
    }

    fn update(&mut self, message: Message) -> iced::Command<Message> {
        //trace!("Message: {:?}", message);
        match message {
            Message::None => Command::none(),
            Message::RefreshIp(ip_result) => {
                trace!("Local IP resolved: {:?}", ip_result);
                self.global_state.local_ip = ip_result;
                Command::none()
            }
            Message::FontLoaded(result) => {
                match result {
                    Ok(n) => trace!("Loaded font {}", n),
                    Err(e) => error!("Failed to load font: {:?}", e),
                }

                Command::none()
            }
            Message::GlobalSettings(message) => global_settings::update(self, message),
            Message::ServerSettings(message) => server_settings::update(self, message),
            Message::MetadataEditor(message) => metadata_editor::update(self, message),
            Message::StopServer(server_id) => {
                trace!("Stop Server {} (Not Implemented)", server_id);
                let server_state = self
                    .get_server_state_mut(server_id)
                    .expect("Failed to look up server state");
                if let RunState::Available(RunData { .. }) = server_state.run_state {
                    server_state.run_state = RunState::Stopping;
                    if let Some(command_channel) = self.monitor_command_channel.to_owned() {
                        Command::perform(
                            send_monitor_command(
                                command_channel,
                                ServerMonitorCommand::KillServer { server_id },
                            ),
                            |_| Message::None,
                        )
                    } else {
                        Command::none()
                    }
                } else {
                    Command::none()
                }
            }
            Message::StartServer(id) => {
                trace!("Start Server {}", id);
                let server_settings = self
                    .get_server_settings(id)
                    .expect("Failed to look up server settings");
                // Write out updated INI files
                if let Err(e) = update_inis_from_settings(
                    &self.config_metadata_state.effective(),
                    &server_settings,
                ) {
                    error!("Failed to save ini files: {}", e.to_string());
                }
                match server::generate_command_line(&self.config_metadata_state, server_settings) {
                    Ok(args) => Command::perform(
                        start_server(
                            id,
                            server_settings.name.clone(),
                            server_settings.get_full_installation_location(),
                            args,
                        ),
                        move |res| match res {
                            Ok(_) => Message::ServerRunStateChanged(id, RunState::Starting),
                            Err(e) => {
                                error!("Failed to start server: {}", e.to_string());
                                Message::ServerRunStateChanged(id, RunState::Stopped)
                            }
                        },
                    ),
                    Err(e) => {
                        error!("Failed to get command line: {}", e.to_string());
                        Command::none()
                    }
                }
            }
            Message::ServerRunStateChanged(server_id, run_state) => {
                trace!("Server Run State Changed {}", server_id);
                let installation_dir = self
                    .get_server_settings(server_id)
                    .expect("Failed to look up server settings")
                    .get_full_installation_location();

                let server_state = self
                    .get_server_state_mut(server_id)
                    .expect("Failed to look up server settings");

                // TODO: If we hit the Starting state, we should start the process monitor for this server.
                // Once we hit the Stopped state, we can stop the process monitor.
                server_state.run_state = run_state.clone();
                if let RunState::Starting = run_state {
                    if let Some(command_channel) = self.monitor_command_channel.to_owned() {
                        Command::perform(
                            send_monitor_command(
                                command_channel,
                                ServerMonitorCommand::AddServer {
                                    server_id,
                                    installation_dir,
                                },
                            ),
                            |_| Message::None,
                        )
                    } else {
                        Command::none()
                    }
                } else {
                    Command::none()
                }
            }

            Message::NewServer => {
                trace!("TODO: New Server");
                let server = Server {
                    settings: ServerSettings {
                        id: Uuid::new_v4(),
                        name: String::new(),
                        installation_location: String::new(),
                        map: "TheIsland_WP".into(),
                        port: 7777,
                        config_entries: ConfigEntries::default(),
                    },
                    state: ServerState::default(),
                };
                self.servers.push(server);

                self.mode = MainWindowMode::EditProfile(ServerSettingsContext {
                    server_id: self.servers.len() - 1,
                    edit_context: server_settings::ServerSettingsEditContext::NotEditing {
                        query: String::new(),
                    },
                });

                Command::none()
            }
            Message::EditServer(id) => {
                trace!("Edit Server {}", id);
                let (id, _) = self
                    .find_server(id)
                    .expect("Failed to look up server settings");
                self.mode = MainWindowMode::EditProfile(ServerSettingsContext {
                    server_id: id,
                    edit_context: server_settings::ServerSettingsEditContext::NotEditing {
                        query: String::new(),
                    },
                });
                Command::none()
            }
            Message::InstallServer(id, mode) => {
                trace!("Install Server {}", id);
                let server_settings = self
                    .get_server_settings(id)
                    .expect("Failed to look up server settings");
                let app_id = self.global_settings.app_id.clone();
                Command::perform(
                    update_server(
                        id,
                        self.global_settings.steamcmd_directory.to_owned(),
                        server_settings.get_full_installation_location(),
                        app_id,
                        mode,
                        self.server_sender_channel.as_ref().unwrap().clone(),
                    ),
                    move |_| Message::ServerUpdated(id),
                )
            }
            Message::ServerUpdated(id) => {
                trace!("Server Updated {}", id);
                let server_state = self
                    .get_server_state_mut(id)
                    .expect("Failed to look up server state");
                server_state.install_state = InstallState::Validating;
                let server_settings = self
                    .get_server_settings(id)
                    .expect("Failed to look up server settings");
                let app_id = self.global_settings.app_id.to_owned();
                Command::perform(
                    validate_server(id, server_settings.get_full_installation_location(), app_id),
                    move |result| {
                        result
                            .map(|r| Message::ServerValidated(id, r))
                            .unwrap_or_else(|e| {
                                Message::ServerValidated(
                                    id,
                                    ValidationResult::Failed(e.to_string()),
                                )
                            })
                    },
                )
            }
            Message::ServerValidated(
                id,
                ValidationResult::Success {
                    version,
                    install_time,
                },
            ) => {
                trace!("Server Validated {}", id);
                let server_state = self
                    .get_server_state_mut(id)
                    .expect("Failed to look up server state");
                server_state.install_state = InstallState::Installed {
                    version,
                    install_time,
                };
                server_state.run_state = RunState::Stopped;
                Command::none()
            }
            Message::ServerValidated(id, ValidationResult::NotInstalled) => {
                trace!("Server not installed {}", id);
                let server_state = self
                    .get_server_state_mut(id)
                    .expect("Failed to look up server state");
                server_state.install_state = InstallState::NotInstalled;
                Command::none()
            }
            Message::ServerValidated(id, ValidationResult::Failed(reason)) => {
                warn!("Server Validation Failed {}", id);
                let server_state = self
                    .get_server_state_mut(id)
                    .expect("Failed to look up server state");
                // TODO: We might want a better status here so we can show something on the card about
                // validation failing, otherwise it might look like the server is gone
                server_state.install_state = InstallState::FailedValidation(reason);
                Command::none()
            }
            Message::Event(_event) => Command::none(),
            // TODO: Extract these to a different location
            Message::AsyncNotification(AsyncNotification::AsyncStarted(sender)) => {
                trace!("Async notification pipe established");

                // Start the server monitor background task
                let (monitor_send, monitor_recv) = channel(100);
                self.server_sender_channel = Some(sender.clone());
                self.monitor_command_channel = Some(monitor_send);

                let mut run_state_commands = Vec::new();

                run_state_commands.push(Command::perform(
                    monitor_server(monitor_recv, sender),
                    |_| Message::None,
                ));

                // Start checking existing servers
                run_state_commands.extend(self.servers.iter().map(|s| {
                    let server_id = s.id();
                    let installation_dir = s.settings.get_full_installation_location();
                    if let Some(command_channel) = self.monitor_command_channel.to_owned() {
                        Command::perform(
                            send_monitor_command(
                                command_channel,
                                ServerMonitorCommand::AddServer {
                                    server_id,
                                    installation_dir,
                                },
                            ),
                            |_| Message::None,
                        )
                    } else {
                        Command::none()
                    }
                }));
                Command::batch(run_state_commands)
            }
            Message::AsyncNotification(AsyncNotification::UpdateServerProgress(id, progress)) => {
                let server_state = self
                    .get_server_state_mut(id)
                    .expect("Failed to look up server state");
                match progress {
                    UpdateServerProgress::Initializing => {
                        server_state.install_state = InstallState::UpdateStarting
                    }
                    UpdateServerProgress::Downloading(progress) => {
                        server_state.install_state = InstallState::Downloading(progress)
                    }
                    UpdateServerProgress::Verifying(progress) => {
                        server_state.install_state = InstallState::Verifying(progress)
                    }
                }

                Command::none()
            }
            Message::AsyncNotification(AsyncNotification::UpdateServerRunState(id, run_state)) => {
                //trace!("UpdateServerRunState {}: {:?}", id, run_state);
                let server_state = self
                    .get_server_state_mut(id)
                    .expect("Failed to look up server state");
                let original_state = server_state.run_state.to_owned();
                server_state.run_state = run_state.to_owned();
                if let RunState::Available(_) = run_state {
                    if let RunState::Stopping = server_state.run_state {
                        server_state.run_state = original_state;
                    }
                }

                Command::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let main_header = components::main_header(&self.global_state);
        let bottom_pane = if let SteamCmdState::Installed = self.global_state.steamcmd_state {
            container(
                column![
                    row![make_button(
                        "New Server",
                        Some(Message::NewServer),
                        icons::ADD.clone()
                    )],
                    if self.servers.is_empty() {
                        container(
                            text("NO SERVERS YET")
                                .font(BOLD_FONT)
                                .size(32)
                                .style(Color::from([0.5, 0.5, 0.5]))
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .vertical_alignment(Vertical::Center)
                                .horizontal_alignment(Horizontal::Center),
                        )
                    } else {
                        container(scrollable(
                            column(self.servers.iter().map(server_card).collect()).spacing(5),
                        ))
                    }
                ]
                .spacing(5)
                .padding(5)
                .width(Length::Fill)
                .height(Length::Fill),
            )
        } else {
            container(
                column![
                    text("SteamCMD not found"),
                    text("Go to Global Settings and find or install it")
                ]
                .align_items(iced::Alignment::Center),
            )
        };

        let mut main_content_children: Vec<Element<_>> = Vec::new();
        if option_env!("IS_RELEASE_BUILD").is_none() {
            main_content_children
                .push(
                    container(text("DEVELOPMENT BUILD - USE AT YOUR OWN RISK").size(15))
                        .style(move |_: &_| container::Appearance {
                            text_color: Some(Color::WHITE),
                            background: Some(iced::Background::Color(Color::from_rgb(
                                1.0, 0.0, 0.0,
                            ))),
                            ..Default::default()
                        })
                        .width(Length::Fill)
                        .align_x(Horizontal::Center)
                        .into(),
                )
                .into()
        }

        main_content_children.push(main_header.into());
        main_content_children.push(horizontal_rule(3).into());
        main_content_children.push(bottom_pane.into());
        let main_content = container(column(main_content_children))
            .width(Length::Fill)
            .height(Length::Fill);

        let result: Element<Message> = match &self.mode {
            MainWindowMode::Servers => main_content.into(),
            MainWindowMode::GlobalSettings => {
                Modal::new(main_content, dialogs::global_settings::make_dialog(&self))
                    .on_blur(GlobalSettingsMessage::CloseGlobalSettings.into())
                    .into()
            }
            MainWindowMode::MetadataEditor(edit_context) => Modal::new(
                main_content,
                dialogs::metadata_editor::make_dialog(&self, edit_context),
            )
            .on_blur(MetadataEditorMessage::CloseMetadataEditor.into())
            .into(),
            MainWindowMode::EditProfile(edit_context) => Modal::new(
                main_content,
                dialogs::server_settings::make_dialog(&self, edit_context),
            )
            .on_blur(ServerSettingsMessage::CloseServerSettings.into())
            .into(),
        };
        if self.global_settings.debug_ui {
            result.explain(Color::BLACK)
        } else {
            result
        }
    }
}

fn main() -> iced::Result {
    init_tracing();

    AppState::run(Settings::default())
}

fn init_tracing() {
    let mut layers = Vec::new();

    let env_filter = EnvFilter::builder()
        .with_default_directive("asma=TRACE".parse().unwrap())
        .from_env()
        .expect("Invalid trace filter specified");
    // let stdout_log = FmtSubscriber::builder()
    //     // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
    //     // will be written to stdout.
    //     .with_max_level(Level::TRACE)
    //     .with_env_filter(env_filter)
    //     // completes the builder.
    //     .finish();

    let stdout_log = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .with_filter(LevelFilter::TRACE)
        .with_filter(env_filter)
        .boxed();
    layers.push(stdout_log);

    // Roll the previous log
    let process_directory = process_path::get_executable_path().expect("Failed to get exe path");

    let asma_log_path = process_directory.with_file_name("asma.log");
    let asma_log_back_path = process_directory.with_file_name("asma.log.bak");

    if std::fs::metadata(&asma_log_path).is_ok() {
        std::fs::rename(&asma_log_path, asma_log_back_path).expect("Failed to rename log file");
    }

    let app_log_file = File::create(asma_log_path).expect("Failed to create log file");
    let env_filter = EnvFilter::builder()
        .with_default_directive("asma=TRACE".parse().unwrap())
        .from_env()
        .expect("Invalid trace filter specified");
    let app_log = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(app_log_file)
        .with_filter(LevelFilter::TRACE)
        .with_filter(env_filter)
        .boxed();
    layers.push(app_log);

    tracing_subscriber::registry().with(layers).init();
    //tracing::subscriber::set_global_default(stdout_log).expect("setting default subscriber failed");
    trace!("Ark Server Manager: Ascended initilizing...");
}
