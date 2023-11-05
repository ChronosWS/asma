use std::fmt::Display;
use std::{net::IpAddr, path::PathBuf};

use iced::widget::{
    self, button, column, container, horizontal_rule, horizontal_space, image, row, text, toggler,
    vertical_space, Container, Row,
};
use iced::{
    executor, subscription, theme, Alignment, Application, Command, Element, Event, Length, Pixels,
    Settings, Subscription, Theme,
};

use steamcmd_utils::get_steamcmd;
use tracing::{error, info, trace, warn, Level};
use tracing_subscriber::{EnvFilter, FmtSubscriber};
use uuid::Uuid;

mod icons;
mod modal;
mod network_utils;
mod steamcmd_utils;

use modal::Modal;

// iced uses a pattern based on the Elm architecture. To implement the pattern, the system is split
// into four parts:
// * The state
// * The messages, which communicate user interactions or events we care about
// * The view logic, which tells the system how to draw and maps user interactions to messages
// * The update logic, which processes messages and updates the state

enum ThemeType {
    Light,
    Dark,
}

struct GlobalSettings {
    theme: ThemeType,
    app_data_directory: String,
    profiles_directory: String,
    steamcmd_directory: String,
}

struct ServerSettings {
    installation_location: String,
}

struct ServerState {
    installed_version: String,
    status: String,
    availability: String,
    current_players: u8,
    max_players: u8,
}

struct ServerProfile {
    id: String,
    name: String,
    settings: ServerSettings,
    state: ServerState,
}

#[derive(Debug, Clone)]
enum LocalIp {
    Unknown,
    Failed,
    Resolving,
    Resolved(IpAddr),
}

impl Display for LocalIp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LocalIp::Unknown => write!(f, "<unknown>"),
            LocalIp::Failed => write!(f, "FAILED"),
            LocalIp::Resolving => write!(f, "Resolving..."),
            LocalIp::Resolved(ip_addr) => write!(f, "{}", ip_addr.to_string()),
        }
    }
}

struct GlobalState {
    app_version: String,
    local_ip: LocalIp,
}

enum MainWindowMode {
    Servers,
    GlobalSettings,
    EditProfile,
}

struct AppState {
    global_settings: GlobalSettings,
    global_state: GlobalState,
    server_profiles: Vec<ServerProfile>,
    mode: MainWindowMode,
}

#[derive(Debug, Clone)]
enum Message {
    RefreshIp(LocalIp),
    OpenGlobalSettings,
    CloseGlobalSettings,

    // Theme
    ThemeToggled(bool),

    // Profiles
    OpenProfilesDirectory,
    SetProfilesDirectory,

    // Steam Messages
    OpenSteamCmdDirectory,
    UpdateSteamCmd,
    SetSteamCmdDirectory,
    SteamCmdUpdated,

    // Keyboard and Mouse events
    Event(Event),
}

impl Application for AppState {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        let local_app_data =
            std::env::var("LOCALAPPDATA").expect("Failed to get LOCALAPPDATA environment variable");

        let app_data_directory = PathBuf::from(format!("{local_app_data}\\ASMAscended"));
        let mut default_profile_directory = app_data_directory.to_owned();
        default_profile_directory.push("Profiles");
        let mut default_steamcmd_directory = app_data_directory.to_owned();
        default_steamcmd_directory.push("SteamCMD");

        std::fs::create_dir_all(default_profile_directory.clone())
            .expect("Failed to create default profile directory");
        std::fs::create_dir_all(default_steamcmd_directory.clone())
            .expect("Failed to create default SteamCMD directory");

        (
            AppState {
                global_settings: GlobalSettings {
                    theme: ThemeType::Dark,
                    app_data_directory: app_data_directory.to_str().unwrap().into(),
                    profiles_directory: default_profile_directory.to_str().unwrap().into(),
                    steamcmd_directory: default_steamcmd_directory.to_str().unwrap().into(),
                },
                global_state: GlobalState {
                    app_version: env!("CARGO_PKG_VERSION").into(),
                    local_ip: LocalIp::Unknown,
                },
                server_profiles: Vec::new(),
                mode: MainWindowMode::Servers,
            },
            Command::perform(network_utils::refresh_ip(), |result| {
                if let Ok(ip_addr) = result {
                    Message::RefreshIp(LocalIp::Resolved(ip_addr))
                } else {
                    Message::RefreshIp(LocalIp::Failed)
                }
            }),
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
        subscription::events().map(Message::Event)
    }

    fn update(&mut self, message: Message) -> iced::Command<Message> {
        //trace!("Message: {:?}", message);
        match message {
            Message::RefreshIp(ip_result) => {
                trace!("Local IP resolved: {:?}", ip_result);
                self.global_state.local_ip = ip_result;
                Command::none()
            }
            Message::OpenGlobalSettings => {
                self.mode = MainWindowMode::GlobalSettings;
                widget::focus_next()
            }
            Message::CloseGlobalSettings => {
                self.mode = MainWindowMode::Servers;
                Command::none()
            }
            Message::UpdateSteamCmd => Command::perform(
                get_steamcmd(self.global_settings.steamcmd_directory.to_owned()),
                |_| Message::SteamCmdUpdated,
            ),
            Message::OpenSteamCmdDirectory => {
                if let Err(e) = std::process::Command::new("explorer")
                    .args([self.global_settings.steamcmd_directory.as_str()])
                    .spawn()
                {
                    error!(
                        "Failed to open {}: {}",
                        self.global_settings.steamcmd_directory,
                        e.to_string()
                    );
                }
                Command::none()
            }
            Message::ThemeToggled(is_dark) => {
                if is_dark {
                    self.global_settings.theme = ThemeType::Dark;
                } else {
                    self.global_settings.theme = ThemeType::Light;
                }
                Command::none()
            }
            Message::SetSteamCmdDirectory => {
                let default_path = self.global_settings.steamcmd_directory.as_str();
                let folder = rfd::FileDialog::new()
                    .set_title("Select SteamCMD directory")
                    .set_directory(default_path)
                    .pick_folder();
                if let Some(folder) = folder {
                    if let Some(folder) = folder.to_str() {
                        info!("Setting path: {}", folder);
                        self.global_settings.steamcmd_directory = folder.into();
                    } else {
                        error!("Failed to convert folder");
                    }
                } else {
                    error!("No folder selected");
                }
                Command::none()
            }
            Message::SteamCmdUpdated => {
                trace!("SteamCmdUpdated");
                Command::none()
            }
            Message::OpenProfilesDirectory => {
                if let Err(e) = std::process::Command::new("explorer")
                    .args([self.global_settings.profiles_directory.as_str()])
                    .spawn()
                {
                    error!(
                        "Failed to open {}: {}",
                        self.global_settings.profiles_directory,
                        e.to_string()
                    );
                }
                Command::none()
            }
            Message::SetProfilesDirectory => {
                let default_path = self.global_settings.profiles_directory.as_str();
                let folder = rfd::FileDialog::new()
                    .set_title("Select SteamCMD directory")
                    .set_directory(default_path)
                    .pick_folder();
                if let Some(folder) = folder {
                    if let Some(folder) = folder.to_str() {
                        info!("Setting path: {}", folder);
                        self.global_settings.profiles_directory = folder.into();
                    } else {
                        error!("Failed to convert folder");
                    }
                } else {
                    error!("No folder selected");
                }
                Command::none()
            }
            Message::Event(_event) => Command::none(),
        }
    }

    fn view(&self) -> Element<Message> {
        let main_content = container(column![self.main_header(), horizontal_rule(3),])
            .width(Length::Fill)
            .height(Length::Fill);

        match self.mode {
            MainWindowMode::Servers => main_content.into(),
            MainWindowMode::GlobalSettings => Modal::new(main_content, self.global_settings())
                .on_blur(Message::CloseGlobalSettings)
                .into(),
            MainWindowMode::EditProfile => main_content.into(),
        }
    }
}

impl AppState {
    fn main_header(&self) -> Row<Message> {
        row![
            column![
                text("ASM: Ascended")
                    .size(40)
                    .vertical_alignment(iced::alignment::Vertical::Top),
                button(row![
                    image::Image::new(icons::SETTINGS.clone())
                        .width(24)
                        .height(24),
                    text("Global Settings...")
                        .vertical_alignment(iced::alignment::Vertical::Center)
                ])
                .on_press(Message::OpenGlobalSettings)
            ],
            horizontal_space(Length::Fill),
            column![
                text("My Public IP"),
                text(self.global_state.local_ip.to_string())
            ]
            .align_items(Alignment::Center),
            horizontal_space(Length::Fill),
            column![
                text("Task Status"),
                text("Auto-Backup: Unknown"),
                text("Auto-Update: Unknown"),
                text("Discord Bot: Disabled"),
            ]
            .align_items(Alignment::Center)
        ]
        .padding(10)
    }

    fn global_settings(&self) -> Container<Message> {
        container(
            column![
                row![
                    text("Global Settings").size(25),
                    horizontal_space(Length::Fill),
                    button(
                        image::Image::new(icons::CANCEL.clone())
                            .width(24)
                            .height(24)
                    )
                    .on_press(Message::CloseGlobalSettings),
                ],
                row![
                    text("Theme:").width(100),
                    text("Light"),
                    toggler(
                        String::new(),
                        match self.global_settings.theme {
                            ThemeType::Light => false,
                            _ => true,
                        },
                        Message::ThemeToggled
                    )
                    .width(Length::Shrink),
                    text("Dark"),
                    horizontal_space(Length::Fill)
                ]
                .spacing(5)
                .height(32),
                row![
                    text("SteamCMD:")
                        .width(100)
                        .vertical_alignment(iced::alignment::Vertical::Center),
                    text(self.global_settings.steamcmd_directory.to_owned())
                        .vertical_alignment(iced::alignment::Vertical::Center),
                    horizontal_space(Length::Fill),
                    button(row![
                        image::Image::new(icons::FOLDER_OPEN.clone())
                            .width(24)
                            .height(24),
                        text("Open...").vertical_alignment(iced::alignment::Vertical::Center)
                    ])
                    .width(100)
                    .padding(3)
                    .on_press(Message::OpenSteamCmdDirectory),
                    button(row![
                        image::Image::new(icons::REFRESH.clone())
                            .width(24)
                            .height(24),
                        text("Update").vertical_alignment(iced::alignment::Vertical::Center)
                    ])
                    .width(100)
                    .padding(3)
                    .on_press(Message::UpdateSteamCmd),
                    button(row![
                        image::Image::new(icons::FOLDER_OPEN.clone())
                            .width(24)
                            .height(24),
                        text("Set Location...")
                            .vertical_alignment(iced::alignment::Vertical::Center)
                    ])
                    .width(150)
                    .padding(3)
                    .on_press(Message::SetSteamCmdDirectory)
                ]
                .spacing(5),
                row![
                    text("Profiles:")
                        .width(100)
                        .vertical_alignment(iced::alignment::Vertical::Center),
                    text(self.global_settings.profiles_directory.to_owned())
                        .vertical_alignment(iced::alignment::Vertical::Center),
                    horizontal_space(Length::Fill),
                    button(row![
                        image::Image::new(icons::FOLDER_OPEN.clone())
                            .width(24)
                            .height(24),
                        text("Open...").vertical_alignment(iced::alignment::Vertical::Center)
                    ])
                    .width(100)
                    .padding(3)
                    .on_press(Message::OpenProfilesDirectory),
                    button(row![
                        image::Image::new(icons::FOLDER_OPEN.clone())
                            .width(24)
                            .height(24),
                        text("Set Location...")
                            .vertical_alignment(iced::alignment::Vertical::Center)
                    ])
                    .width(150)
                    .padding(3)
                    .on_press(Message::SetProfilesDirectory)
                ]
                .spacing(5)
            ]
            .spacing(5),
        )
        .padding(10)
        .style(theme::Container::Box)
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
        .parse("asma=TRACE")
        .expect("Bad tracing filter");
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
