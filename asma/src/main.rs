use std::sync::Arc;

use components::{make_button, server_card};
use config_utils::create_metadata_index;
use dialogs::global_settings::{self, GlobalSettingsMessage};
use dialogs::metadata_editor::{self, MetadataEditorMessage, MetadataEditContext};
use dialogs::server_settings::{self, ServerSettingsMessage};
use fonts::{get_system_font_bytes, BOLD_FONT};
use futures_util::SinkExt;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{column, container, horizontal_rule, row, scrollable, text};
use iced::{
    executor, font, subscription, Application, Color, Command, Element, Event, Length, Settings,
    Subscription, Theme,
};

use models::config::{ConfigEntries, ConfigMetadata};
use server_utils::{UpdateServerProgress, ValidationResult};
use steamcmd_utils::validate_steamcmd;
use sysinfo::{System, SystemExt};
use tantivy::Index;
use tokio::sync::mpsc::{channel, Sender};
use tokio::sync::Mutex;
use tracing::{error, trace, warn, Level};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

mod components;
mod config_utils;
mod dialogs;
mod file_utils;
mod fonts;
mod icons;
mod modal;
mod models;
mod network_utils;
mod server_utils;
mod settings_utils;
mod steamcmd_utils;

use modal::Modal;
use models::*;
use uuid::Uuid;

use crate::server_utils::{
    monitor_server, start_server, stop_server, update_server, validate_server, UpdateMode,
};

// iced uses a pattern based on the Elm architecture. To implement the pattern, the system is split
// into four parts:
// * The state
// * The messages, which communicate user interactions or events we care about
// * The view logic, which tells the system how to draw and maps user interactions to messages
// * The update logic, which processes messages and updates the state

enum MainWindowMode {
    Servers,
    GlobalSettings,
    EditProfile(Uuid),
    MetadataEditor(MetadataEditContext),
}

struct AppState {
    async_sender: Option<Sender<AsyncNotification>>,
    system: Arc<Mutex<System>>,
    global_settings: GlobalSettings,
    global_state: GlobalState,
    config_metadata: ConfigMetadata,
    config_index: Index,
    servers: Vec<Server>,
    mode: MainWindowMode,
}

impl AppState {
    // TODO: These should probably just be changed to `get_server*` since settings
    // and state often go together and interior mutability is a PITA.
    pub fn get_server_settings(&self, id: Uuid) -> Option<&ServerSettings> {
        self.servers
            .iter()
            .find(|s| s.settings.id == id)
            .map(|s| &s.settings)
    }
    pub fn get_server_settings_mut(&mut self, id: Uuid) -> Option<&mut ServerSettings> {
        self.servers
            .iter_mut()
            .find(|s| s.settings.id == id)
            .map(|s| &mut s.settings)
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
        let config_metadata = config_utils::load_config_metadata().unwrap_or_default();
        let servers = settings_utils::load_server_settings(&global_settings)
            .expect("Failed to load server settings")
            .drain(..)
            .map(|settings| Server {
                settings,
                state: ServerState::default(),
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

        

        (
            AppState {
                async_sender: None,
                system: Arc::new(Mutex::new(System::default())),
                global_settings,
                global_state: GlobalState {
                    app_version: env!("CARGO_PKG_VERSION").into(),
                    local_ip: LocalIp::Unknown,
                    edit_server_id: None,
                    edit_metadata_id: None,
                    steamcmd_state,
                },
                config_metadata,
                config_index: create_metadata_index(),
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
            Message::StopServer(id) => {
                trace!("Stop Server {} (Not Implemented)", id);
                let server_state = self
                    .get_server_state_mut(id)
                    .expect("Failed to look up server state");
                if let RunState::Available(RunData { pid, .. }) = server_state.run_state {
                    server_state.run_state = RunState::Stopping;
                    Command::perform(stop_server(id, pid, self.system.clone()), move |_| {
                        Message::None
                    })
                } else {
                    Command::none()
                }
            }
            Message::StartServer(id) => {
                trace!("Start Server {}", id);
                let server_settings = self
                    .get_server_settings(id)
                    .expect("Failed to look up server settings");
                Command::perform(
                    start_server(
                        id,
                        server_settings.name.clone(),
                        server_settings.get_full_installation_location(),
                        server_settings.map.clone(),
                        server_settings.port,
                    ),
                    move |res| match res {
                        Ok(_) => Message::ServerRunStateChanged(id, RunState::Starting),
                        Err(e) => {
                            error!("Failed to start server: {}", e.to_string());
                            Message::ServerRunStateChanged(id, RunState::Stopped)
                        }
                    },
                )
            }
            Message::ServerRunStateChanged(id, run_state) => {
                trace!("Server Run State Changed {}", id);
                let installation_dir = self
                    .get_server_settings(id)
                    .expect("Failed to look up server settings")
                    .get_full_installation_location();

                let server_state = self
                    .get_server_state_mut(id)
                    .expect("Failed to look up server settings");

                // TODO: If we hit the Starting state, we should start the process monitor for this server.
                // Once we hit the Stopped state, we can stop the process monitor.
                server_state.run_state = run_state.clone();
                if let RunState::Starting = run_state {
                    Command::perform(
                        monitor_server(
                            id,
                            installation_dir,
                            self.system.clone(),
                            self.async_sender.as_ref().unwrap().clone(),
                        ),
                        move |res| match res {
                            Ok(_) => Message::ServerRunStateChanged(id, RunState::Stopped),
                            Err(e) => {
                                error!(
                                    "Failed to get server process information: {}",
                                    e.to_string()
                                );
                                Message::ServerRunStateChanged(id, RunState::Stopped)
                            }
                        },
                    )
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
                        config_entries: ConfigEntries::default()
                    },
                    state: ServerState::default(),
                };
                self.mode = MainWindowMode::EditProfile(server.settings.id);

                //self.global_state.edit_server_id = Some(server.settings.id);
                self.servers.push(server);
                Command::none()
            }
            Message::EditServer(id) => {
                trace!("Edit Server {}", id);
                let edit_server = self
                    .get_server_settings(id)
                    .expect("Failed to look up server settings");
                self.global_state.edit_server_id = Some(edit_server.id);
                self.mode = MainWindowMode::EditProfile(id);
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
                        self.async_sender.as_ref().unwrap().clone(),
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
            Message::ServerValidated(id, ValidationResult::Success(version)) => {
                trace!("Server Validated {}", id);
                let server_state = self
                    .get_server_state_mut(id)
                    .expect("Failed to look up server state");
                server_state.install_state = InstallState::Installed(version);
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
                self.async_sender = Some(sender);

                // Run deferred startup operations
                let run_state_commands = self.servers.iter().map(|s| {
                    let id = s.id();
                    let installation_dir = s.settings.get_full_installation_location();
                    Command::perform(
                        monitor_server(
                            id,
                            installation_dir,
                            self.system.clone(),
                            self.async_sender.as_ref().unwrap().clone(),
                        ),
                        move |res| match res {
                            Ok(_) => Message::ServerRunStateChanged(id, RunState::Stopped),
                            Err(e) => {
                                error!(
                                    "Failed to get server process information: {}",
                                    e.to_string()
                                );
                                Message::ServerRunStateChanged(id, RunState::Stopped)
                            }
                        },
                    )
                });
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
                server_state.run_state = run_state;
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
                        Message::NewServer,
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

        let main_content = container(column![main_header, horizontal_rule(3), bottom_pane])
            .width(Length::Fill)
            .height(Length::Fill);

        let result: Element<Message> = match &self.mode {
            MainWindowMode::Servers => main_content.into(),
            MainWindowMode::GlobalSettings => {
                Modal::new(main_content, dialogs::global_settings::make_dialog(&self))
                    .on_blur(GlobalSettingsMessage::CloseGlobalSettings.into())
                    .into()
            }
            MainWindowMode::MetadataEditor(edit_context) => {
                Modal::new(main_content, dialogs::metadata_editor::make_dialog(&self, edit_context))
                    .on_blur(MetadataEditorMessage::CloseMetadataEditor.into())
                    .into()
            }
            MainWindowMode::EditProfile(server_id) => {
                Modal::new(
                    main_content,
                    dialogs::server_settings::make_dialog(&self, *server_id),
                )
                .on_blur(ServerSettingsMessage::CloseServerSettings(*server_id).into())
                .into()
            }
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
    let env_filter = EnvFilter::builder()
        .with_default_directive("asma=TRACE".parse().unwrap())
        .from_env()
        .expect("Invalid trace filter specified");
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::TRACE)
        .with_env_filter(env_filter)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    trace!("Ark Server Manager: Ascended initilizing...");
}
