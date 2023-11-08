use components::{make_button, server_card};
use dialogs::global_settings::{self, GlobalSettingsMessage};
use dialogs::server_settings::{self, ServerSettingsMessage};
use fonts::{get_system_font_bytes, BOLD_FONT};
use futures_util::SinkExt;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{column, container, horizontal_rule, row, scrollable, text};
use iced::{
    executor, font, subscription, Application, Color, Command, Element, Event, Length, Settings,
    Subscription, Theme,
};

use server_utils::UpdateServerProgress;
use tokio::sync::mpsc::{channel, Sender};
use tracing::{error, trace, Level};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

mod components;
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

use crate::server_utils::{update_server, UpdateMode};

// iced uses a pattern based on the Elm architecture. To implement the pattern, the system is split
// into four parts:
// * The state
// * The messages, which communicate user interactions or events we care about
// * The view logic, which tells the system how to draw and maps user interactions to messages
// * The update logic, which processes messages and updates the state

enum MainWindowMode {
    Servers,
    GlobalSettings,
    EditProfile,
}

struct AppState {
    async_sender: Option<Sender<AsyncNotification>>,
    global_settings: GlobalSettings,
    global_state: GlobalState,
    servers: Vec<Server>,
    mode: MainWindowMode,
}

impl AppState {
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
}

#[derive(Debug, Clone)]
pub enum Message {
    FontLoaded(Result<String, font::Error>),
    RefreshIp(LocalIp),

    GlobalSettings(GlobalSettingsMessage),
    ServerSettings(ServerSettingsMessage),

    // Servers
    NewServer,
    EditServer(Uuid),
    InstallServer(Uuid),
    ServerUpdated(Uuid),

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
        let arial_bytes = get_system_font_bytes("ARIAL.ttf").expect("Failed to find Arial");
        let global_settings = settings_utils::load_global_settings()
            .unwrap_or_else(|_| settings_utils::default_global_settings());
        let servers = settings_utils::load_server_settings(&global_settings)
            .expect("Failed to load server settings")
            .drain(..)
            .map(|settings| Server {
                settings,
                state: ServerState::default(),
            })
            .collect();

        (
            AppState {
                async_sender: None,
                global_settings,
                global_state: GlobalState {
                    app_version: env!("CARGO_PKG_VERSION").into(),
                    local_ip: LocalIp::Unknown,
                    edit_server_id: Uuid::nil(),
                },
                servers,
                mode: MainWindowMode::Servers,
            },
            Command::batch(vec![
                font::load(std::borrow::Cow::from(arial_bytes))
                    .map(|v| Message::FontLoaded(v.map(|_| "Arial".into()))),
                Command::perform(network_utils::refresh_ip(), |result| {
                    if let Ok(ip_addr) = result {
                        Message::RefreshIp(LocalIp::Resolved(ip_addr))
                    } else {
                        Message::RefreshIp(LocalIp::Failed)
                    }
                }),
            ]),
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
            subscription::events().map(Message::Event),
            async_pump().map(Message::AsyncNotification),
        ])
    }

    fn update(&mut self, message: Message) -> iced::Command<Message> {
        //trace!("Message: {:?}", message);
        match message {
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
            Message::NewServer => {
                trace!("TODO: New Server");
                self.mode = MainWindowMode::EditProfile;
                let server = Server {
                    settings: ServerSettings {
                        id: Uuid::new_v4(),
                        name: String::new(),
                        installation_location: String::new(),
                    },
                    state: ServerState::default(),
                };
                self.global_state.edit_server_id = server.settings.id;
                self.servers.push(server);
                Command::none()
            }
            Message::EditServer(id) => {
                trace!("Edit Server {}", id.to_string());
                self.mode = MainWindowMode::EditProfile;
                let edit_server = self
                    .get_server_settings(id)
                    .expect("Failed to look up server settings");
                self.global_state.edit_server_id = edit_server.id;
                Command::none()
            }
            Message::InstallServer(id) => {
                trace!("Install Server {}", id.to_string());
                let server_settings = self
                    .get_server_settings(id)
                    .expect("Failed to look up server settings");
                Command::perform(
                    update_server(
                        id,
                        self.global_settings.steamcmd_directory.to_owned(),
                        server_settings.get_full_installation_location(),
                        "2430930",
                        UpdateMode::Update,
                        self.async_sender.as_ref().unwrap().clone(),
                    ),
                    move |_| Message::ServerUpdated(id),
                )
            }
            Message::ServerUpdated(id) => {
                trace!("Server Updated {}", id.to_string());
                let server_state = self
                    .get_server_state_mut(id)
                    .expect("Failed to look up server state");
                server_state.install_state = InstallState::Installed("<unknown>".into());
                Command::none()
            }
            Message::Event(_event) => Command::none(),
            // TODO: Extract these to a different location
            Message::AsyncNotification(AsyncNotification::AsyncStarted(sender)) => {
                trace!("Async notification pipe established");
                self.async_sender = Some(sender);
                Command::none()
            }
            Message::AsyncNotification(AsyncNotification::UpdateServerProgress(id, progress)) => {
                let server_state = self
                    .get_server_state_mut(id)
                    .expect("Failed to look up server state");
                trace!("Server Progress: {:?}", progress);
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
        }
    }

    fn view(&self) -> Element<Message> {
        let main_header = components::main_header(&self.global_state);

        let servers_list = column![
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
            } // if self.servers.is_empty() {
              // } else {
              //     scrollable(column![]).into()
              // }
        ]
        .spacing(5)
        .padding(5)
        .width(Length::Fill)
        .height(Length::Fill);

        let main_content = container(column![main_header, horizontal_rule(3), servers_list])
            .width(Length::Fill)
            .height(Length::Fill);

        let result: Element<Message> = match self.mode {
            MainWindowMode::Servers => main_content.into(),
            MainWindowMode::GlobalSettings => Modal::new(
                main_content,
                dialogs::global_settings::make_dialog(&self.global_settings),
            )
            .on_blur(GlobalSettingsMessage::CloseGlobalSettings.into())
            .into(),
            MainWindowMode::EditProfile => {
                let server_settings = self
                    .get_server_settings(self.global_state.edit_server_id)
                    .expect("Non-existant server requested for edit");
                Modal::new(
                    main_content,
                    dialogs::server_settings::make_dialog(server_settings),
                )
                .on_blur(ServerSettingsMessage::CloseServerSettings(server_settings.id).into())
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

    AppState::run(Settings {
        ..Default::default()
    })
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
