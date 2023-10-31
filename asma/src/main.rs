use std::{net::IpAddr, sync::Arc};

use once_cell::sync::Lazy;
use tokio::runtime::{Builder, Runtime};

slint::include_modules!();

async fn refresh_ip() -> Result<IpAddr, ()> {
    let mut response = reqwest::get("https://api.ipify.org").await.map_err(|e| {
        eprintln!(
            "Error requesting IP from https://api.ipify.org: {}",
            e.to_string()
        )
    });

    if response.is_err() {
        response = reqwest::get("http://whatismyip.akamai.com")
            .await
            .map_err(|e| {
                eprintln!(
                    "Error requesting IP from http://whatismyip.akamai.com: {}",
                    e.to_string()
                )
            })
    }

    if let Ok(response) = response {
        if let Ok(text) = response
            .text()
            .await
            .map_err(|e| eprintln!("Failed to get response value: {}", e.to_string()))
        {
            return text.parse::<IpAddr>().map_err(|e| {
                eprintln!(
                    "Failed to parse IP address from response '{}': {}",
                    text,
                    e.to_string()
                )
            });
        }
    }
    Err(())
}

static ASYNC_RUNTIME: Lazy<Arc<Runtime>> = Lazy::new(|| {
    Arc::new(
        Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap(),
    )
});

// NOTES on async
// See https://tokio.rs/tokio/topics/bridging
fn main() -> Result<(), slint::PlatformError> {
    // Set up the Tokio runtime to perform async operations on other threads.
    // NOTE: The main thread is reserved for UI operations for platform reasons.
    // See https://slint.dev/releases/1.2.2/docs/rust/slint/#threading-and-event-loop.
    // Therefore any async operations must be performed elsewhere and their values
    // returned to the main thread via something like `slint::invoke_from_event_loop`.

    // Launch the Slint UI
    let ui = AppWindow::new()?;

    // This ui_handle can be cloned and passed around to different threads
    let ui_handle = ui.as_weak();

    let ui = ui_handle.unwrap();
    ui.set_version(env!("CARGO_PKG_VERSION").into());
    ui.set_profile(Profile {
        id: "Test Profile".into(),
        availability: "Unavailable".into(),
        current_players: 0,
        installation_location: "".into(),
        installed_version: "0.0".into(),
        max_players: 70,
        name: "Unnamed Profile".into(),
        status: "Not Installed".into(),
    });

    // NOTE: Async functions must be run on a different thread under tokio.  To accomplish this
    // we need to clone the weak handle (NOTE: if we have multiple handlers, we may need to do the clone
    // outside of the callback due to the move), make the async call, and then invoke the UI updating functions
    // back on the main thread using `upgrade_in_event_loop`.
    // TODO: Error handling - we are currently just printing errors to the console, we should gather those up from
    // our async calls and log them somewhere, and possibly raise a dialog box for those operations which require it
    // TODO: Boilerplate - most of this call is boilerplate. Possibly a good candidate for a macro.
    ui.on_refresh_ip(move || {
        println!("refresh_ip pressed!");
        let ui_handle = ui_handle.clone();
        ASYNC_RUNTIME.spawn(async move {
            let ip = refresh_ip().await;
            if let Ok(ip) = ip {
                ui_handle
                .upgrade_in_event_loop(move |handle| handle.set_ip(ip.to_string().into()))
                .map_err(|e| eprintln!("Failed to execute callback in event loop: {}", e.to_string()))
            } else {
                Err(())
            }
        });
    });
    
    // This starts the event loop and doesn't return until the application window is closed.
    ui.run()
}
