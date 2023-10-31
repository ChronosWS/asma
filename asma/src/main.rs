slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;

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
    // ui.on_request_increase_value(move || {

    // });

    ui.run()
}
