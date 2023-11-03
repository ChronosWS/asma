use std::{path::PathBuf, sync::Arc};

use network_utils::refresh_ip;
use once_cell::sync::Lazy;
use steamcmd_utils::get_steamcmd;
use tokio::runtime::{Builder, Runtime};
use tracing::{error, info, trace, Level};
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;

slint::include_modules!();
mod network_utils;
mod steamcmd_utils;

// NOTES on async
// See https://tokio.rs/tokio/topics/bridging
// Set up the Tokio runtime to perform async operations on other threads.
// NOTE: The main thread is reserved for UI operations for platform reasons.
// See https://slint.dev/releases/1.2.2/docs/rust/slint/#threading-and-event-loop.
// Therefore any async operations must be performed elsewhere and their values
// returned to the main thread via something like `slint::invoke_from_event_loop`.
static ASYNC_RUNTIME: Lazy<Arc<Runtime>> = Lazy::new(|| {
    Arc::new(
        Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap(),
    )
});

fn main() -> Result<(), slint::PlatformError> {
    init_tracing();

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

    // Launch the Slint UI
    let app_window = AppWindow::new().expect("Failed to create main application window");

    // This ui_handle can be cloned and passed around to different threads
    let app_window_weak = app_window.as_weak();

    app_window
        .global::<GlobalConfiguration>()
        .set_version(env!("CARGO_PKG_VERSION").into());
    app_window
        .global::<GlobalConfiguration>()
        .set_local_ip("<unknown>".into());

    app_window
        .global::<GlobalConfiguration>()
        .set_app_data_directory(
            app_data_directory
                .to_str()
                .expect("Unable to format app data directory")
                .into(),
        );
    app_window
        .global::<GlobalConfiguration>()
        .set_profiles_directory(
            default_profile_directory
                .to_str()
                .expect("Unable to format default profiles directory")
                .into(),
        );
    app_window
        .global::<GlobalConfiguration>()
        .set_steamcmd_directory(
            default_steamcmd_directory
                .to_str()
                .expect("Unable to format default steamcmd directory")
                .into(),
        );

    app_window.set_server_profiles(
        [ServerProfile {
            id: Uuid::new_v4().to_string().into(),
            name: "Test Profile".into(),
            settings: ServerSettings {
                installation_location: "".into(),
            },
            state: ServerState {
                availability: "Unavailable".into(),
                current_players: 0,
                installed_version: "0.0".into(),
                max_players: 70,
                status: "Not Installed".into(),
            },
        }]
        .into(),
    );

    // slint::slint! {
    //     import { GlobalSettings } from "ui/windows/global_settings.slint";

    //     export component GlobalSettingsWindow inherits Window {
    //         callback set_steamcmd_location;
    //         callback update_steamcmd;

    //         width: 768px;
    //         GlobalSettings {
    //             set-steamcmd-location => { set_steamcmd_location() }
    //             update-steamcmd => { update_steamcmd() }
    //         }
    //     }
    // }

    // let global_settings_window =
    //     GlobalSettingsWindow::new().expect("Failed to create GlobalSettingsWindow");

    // OnSetSteamCmdLocation
    app_window.on_set_steamcmd_location({
        let app_window_weak = app_window.as_weak();
        move || {
            let app_window = app_window_weak.unwrap();
            let default_path = app_window
                .global::<GlobalConfiguration>()
                .get_steamcmd_directory();
            info!("Default path: {}", default_path);
            let folder = rfd::FileDialog::new()
                .set_title("Select SteamCMD directory")
                .set_directory(default_path.as_str())
                .pick_folder();
            if let Some(folder) = folder {
                if let Some(folder) = folder.to_str() {
                    info!("Setting path: {}", folder);
                    app_window
                        .global::<GlobalConfiguration>()
                        .set_steamcmd_directory(folder.into());
                    let set_path = app_window
                        .global::<GlobalConfiguration>()
                        .get_steamcmd_directory();
                    info!("Path after setting: {}", set_path);
                } else {
                    error!("Failed to convert folder");
                }
            } else {
                error!("No folder selected");
            }
        }
    });

    // OnUpdateSteamCmd
    {
        let app_window_weak = app_window.as_weak();
        app_window.on_update_steamcmd(move || {
            let app_window_weak = app_window_weak.clone();
            let destination_path = app_window_weak.unwrap().global::<GlobalConfiguration>().get_steamcmd_directory().to_string();
            ASYNC_RUNTIME.spawn(async move {
                if let Err(error) = get_steamcmd(destination_path).await {
                    error!("Failed to get steamcmd: {}", error);
                    return;
                }
                trace!("steamcmd updated");
            });
        });
    }

    // Refresh our IP now
    {
        let app_window_weak = app_window_weak.clone();
        ASYNC_RUNTIME.spawn(async move {
            let _ = app_window_weak
                .upgrade_in_event_loop(move |handle| {
                    handle
                        .global::<GlobalConfiguration>()
                        .set_local_ip("... resolving ...".into());
                })
                .map_err(|e| {
                    eprintln!(
                        "Failed to execute callback in event loop: {}",
                        e.to_string()
                    )
                });
            let ip = refresh_ip().await;
            if let Ok(ip) = ip {
                app_window_weak
                    .upgrade_in_event_loop(move |handle| {
                        handle
                            .global::<GlobalConfiguration>()
                            .set_local_ip(ip.to_string().into());
                    })
                    .map_err(|e| {
                        eprintln!(
                            "Failed to execute callback in event loop: {}",
                            e.to_string()
                        )
                    })
            } else {
                Err(())
            }
        });
    }

    // global_settings_window.on_update_steamcmd(move || {});

    // NOTE: Async functions must be run on a different thread under tokio.  To accomplish this
    // we need to clone the weak handle (NOTE: if we have multiple handlers, we may need to do the clone
    // outside of the callback due to the move), make the async call, and then invoke the UI updating functions
    // back on the main thread using `upgrade_in_event_loop`.
    // TODO: Error handling - we are currently just printing errors to the console, we should gather those up from
    // our async calls and log them somewhere, and possibly raise a dialog box for those operations which require it
    // TODO: Boilerplate - most of this call is boilerplate. Possibly a good candidate for a macro.

    // NOTE: Clone here because we will move `ui_handle2` into the first delegate
    // I wonder if we can store a static version of the weak handle and clone it wherever we need it...

    // NOTE: Second clone here because of the following error if we don't:
    // error[E0507]: cannot move out of `ui_handle2`, a captured variable in an `FnMut` closure
    //    --> asma\src\main.rs:97:29
    //     |
    // 92  |       let ui_handle2 = ui_handle.clone();
    //     |           ---------- captured outer variable
    // 93  |       ui.on_refresh_ip(move || {
    //     |                        ------- captured by this `FnMut` closure
    // ...
    // 97  |           ASYNC_RUNTIME.spawn(async move {
    //     |  _____________________________^
    // 98  | |             let ip = refresh_ip().await;
    // 99  | |             if let Ok(ip) = ip {
    // 100 | |                 ui_handle2
    //     | |                 ----------
    //     | |                 |
    //     | |                 variable moved due to use in generator
    //     | |                 move occurs because `ui_handle2` has type `slint::Weak<AppWindow>`, which does not implement the `Copy` trait
    // ...   |
    // 110 | |             }
    // 111 | |         });
    //     | |_________^ `ui_handle2` is moved here

    // let ui_handle2 = ui_handle.clone();
    // ui.on_profile_set_location(move || {
    //     println!("profile_set_location pressed!");
    //     let folder = rfd::FileDialog::new()
    //         .set_title("Select installation directory")
    //         .pick_folder();
    //     if let Some(folder) = folder {
    //         if let Some(folder) = folder.to_str() {
    //             let ui = ui_handle2.unwrap();
    //             let mut profile = ui.get_profile();
    //             profile.settings.installation_location = folder.into();
    //             ui.set_profile(profile);
    //         }
    //     }
    // });

    // let ui_handle2 = ui_handle.clone();
    // ui.on_profile_install(move || {
    //     println!("profile_install pressed!");
    //     let ui = ui_handle2.unwrap();
    //     let profile = ui.get_profile();
    //     if std::fs::create_dir_all(profile.settings.installation_location.as_str()).is_ok() {
    //         let steamcmd = std::process::Command::new("E:\\Games\\SteamCMD\\steamcmd.exe")
    //             .args([
    //                 "+force_install_dir",
    //                 profile.settings.installation_location.as_str(),
    //                 "+login",
    //                 "anonymous",
    //                 "+app_update",
    //                 "2430930",
    //                 "validate",
    //                 "+quit",
    //             ])
    //             .stdout(std::process::Stdio::inherit())
    //             .spawn()
    //             .map_err(|e| eprintln!("Failed to start steamcmd: {}", e.to_string()));
    //         if let Ok(mut steamcmd) = steamcmd {
    //             let result = steamcmd
    //                 .wait()
    //                 .map_err(|e| eprintln!("steamcmd errored: {}", e.to_string()));
    //             if let Ok(exit_status) = result {
    //                 println!("steamcmd exited with code {}", exit_status.to_string());
    //             }
    //         }
    //     } else {
    //         eprintln!(
    //             "Failed to create directory: {}",
    //             profile.settings.installation_location
    //         );
    //     }
    // });

    // This starts the event loop and doesn't return until the application window is closed.
    app_window.run()
}

fn init_tracing() {
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::TRACE)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    trace!("Ark Server Manager: Ascended initilizing...");
}
