use components::make_button;
use fonts::{get_system_font_bytes, BOLD_FONT, ITALIC_FONT};
use iced::alignment::{Vertical, Horizontal};
use iced::widget::{self, column, container, horizontal_rule, row, scrollable, text};
use iced::{
    executor, font, subscription, Application, Color, Command, Element, Event, Length, Settings,
    Subscription, Theme,
};

use steamcmd_utils::get_steamcmd;
use tracing::{error, info, trace, Level};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

mod components;
mod dialogs;
mod fonts;
mod icons;
mod modal;
mod models;
mod network_utils;
mod settings_utils;
mod steamcmd_utils;

use modal::Modal;
use models::*;

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
    global_settings: GlobalSettings,
    global_state: GlobalState,
    servers: Vec<Server>,
    mode: MainWindowMode,
}

#[derive(Debug, Clone)]
pub enum Message {
    FontLoaded(Result<String, font::Error>),
    RefreshIp(LocalIp),

    // Global Settings
    OpenGlobalSettings,
    CloseGlobalSettings,

    // Theme
    ThemeToggled(bool),
    DebugUIToggled(bool),

    // Profiles
    OpenProfilesDirectory,
    SetProfilesDirectory,

    // Steam Messages
    OpenSteamCmdDirectory,
    UpdateSteamCmd,
    SetSteamCmdDirectory,
    SteamCmdUpdated,

    // Servers
    NewServer,

    // Keyboard and Mouse events
    Event(Event),
}

impl Application for AppState {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        let arial_bytes = get_system_font_bytes("ARIAL.ttf").expect("Failed to find Arial");
        let bold_bytes = get_system_font_bytes("ARIALBD.ttf").expect("Failed to find Arial Bold");
        (
            AppState {
                global_settings: settings_utils::load_global_settings()
                    .unwrap_or_else(|_| settings_utils::default_global_settings()),
                global_state: GlobalState {
                    app_version: env!("CARGO_PKG_VERSION").into(),
                    local_ip: LocalIp::Unknown,
                },
                servers: Vec::new(),
                mode: MainWindowMode::Servers,
            },
            Command::batch(vec![
                font::load(std::borrow::Cow::from(arial_bytes))
                    .map(|v| Message::FontLoaded(v.map(|_| "Arial".into()))),
                // font::load(std::borrow::Cow::from(bold_bytes))
                //     .map(|v| Message::FontLoaded(v.map(|_| "Arial Bold".into()))),
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
            Message::FontLoaded(result) => {
                match result {
                    Ok(n) => trace!("Loaded font {}", n),
                    Err(e) => error!("Failed to load font: {:?}", e),
                }

                Command::none()
            }
            Message::OpenGlobalSettings => {
                self.mode = MainWindowMode::GlobalSettings;
                widget::focus_next()
            }
            Message::CloseGlobalSettings => {
                self.mode = MainWindowMode::Servers;
                let _ = settings_utils::save_global_settings(&self.global_settings)
                    .map_err(|e| error!("Failed to save global settings: {}", e.to_string()));
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
            Message::DebugUIToggled(enable) => {
                self.global_settings.debug_ui = enable;
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
            Message::NewServer => {
                trace!("TODO: New Server");
                Command::none()
            }
            Message::Event(_event) => Command::none(),
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
            text("NO SERVERS YET")
                .font(BOLD_FONT)
                .size(32)
                .style(Color::from([0.5, 0.5, 0.5]))
                .width(Length::Fill)
                .height(Length::Fill)
                .vertical_alignment(Vertical::Center)
                .horizontal_alignment(Horizontal::Center)
                 // if self.servers.is_empty() {
                // } else {
                //     scrollable(column![]).into()
                // }
        ]
        .width(Length::Fill)
        .height(Length::Fill);

        let main_content = container(column![main_header, horizontal_rule(3), servers_list])
            .width(Length::Fill)
            .height(Length::Fill);

        let result: Element<Message> = match self.mode {
            MainWindowMode::Servers => main_content.into(),
            MainWindowMode::GlobalSettings => Modal::new(
                main_content,
                dialogs::global_settings(&self.global_settings),
            )
            .on_blur(Message::CloseGlobalSettings)
            .into(),
            MainWindowMode::EditProfile => main_content.into(),
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
