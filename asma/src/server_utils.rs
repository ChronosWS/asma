use anyhow::{Result, bail};
use tracing::error;
use std::{path::Path, process};

pub enum UpdateMode {
    Update,
    Validate,
}

pub async fn update_server(
    steamcmd_dir: impl AsRef<str>,
    installation_dir: impl AsRef<str>,
    app_id: impl AsRef<str>,
    mode: UpdateMode,
) -> Result<()> {
    let steamcmd_dir = steamcmd_dir.as_ref();
    let installation_dir = installation_dir.as_ref();
    let steamcmd_exe = Path::new(steamcmd_dir).join("steamcmd.exe");
    let mut command = process::Command::new(steamcmd_exe);

    let mut args = vec![
        "+force_install_dir",
        installation_dir,
        "+login",
        "anonymous",
        "+app_update",
        app_id.as_ref(),
    ];
    if let UpdateMode::Validate = mode {
        args.push("validate");
    }

    args.push("quit");

    command.args(args);
    let mut child = command.spawn()?;
    let result = child.wait()?;
    if result.success() {
        Ok(())
    } else {
        error!("steamcmd failed {:?}: {:?}", command, result.code());
        bail!("steamcmd failed {:?}: {}", command, result)
    }
}