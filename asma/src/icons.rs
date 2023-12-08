use iced::widget::image;
use static_init::dynamic;
#[dynamic]
pub static LOGO: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/logo.png"));
#[dynamic]
pub static APP_ICON: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/DinoHead.png"));
#[dynamic]
pub static ASMA_STEVE: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/ASMA_SteveLastics.png"));
#[dynamic]
pub static ADD: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Add.ico"));
#[dynamic]
pub static CANCEL: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Cancel.ico"));
#[dynamic]
pub static DELETE: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Delete.ico"));
#[dynamic]
pub static DOWN: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Down.ico"));
#[dynamic]
pub static DOWNLOAD: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Download.ico"));
#[dynamic]
pub static EDIT: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Edit.ico"));
#[dynamic]
pub static FOLDER_DELETE: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/FolderDelete.ico"));
#[dynamic]
pub static FOLDER_OPEN: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/FolderOpen.ico"));
#[dynamic]
pub static LOGS: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Logs.ico"));
#[dynamic]
pub static REFRESH: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Refresh.ico"));
#[dynamic]
pub static RELOAD: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Reload.ico"));
#[dynamic]
pub static SAVE: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Save.ico"));
#[dynamic]
pub static SETTINGS: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Settings.ico"));
#[dynamic]
pub static START: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Start.ico"));
#[dynamic]
pub static STOP: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Stop.ico"));
#[dynamic]
pub static UP: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Up.ico"));
#[dynamic]
pub static VALIDATE: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Validate.ico"));
