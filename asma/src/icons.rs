use iced::widget::image;
use static_init::dynamic;

#[dynamic]
pub static LOGO: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/logo.png")).into();
#[dynamic]
pub static APP_ICON: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/DinoHead.png")).into();
#[dynamic]
pub static ASMA_STEVE: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/ASMA_SteveLastics.png")).into();
#[dynamic]
pub static ADD: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Add.ico")).into();
#[dynamic]
pub static CANCEL: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Cancel.ico")).into();
#[dynamic]
pub static DELETE: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Delete.ico")).into();
#[dynamic]
pub static DOWN: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Down.ico")).into();
#[dynamic]
pub static DOWNLOAD: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Download.ico")).into();
#[dynamic]
pub static EDIT: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Edit.ico")).into();
#[dynamic]
pub static FOLDER_DELETE: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/FolderDelete.ico")).into();
#[dynamic]
pub static FOLDER_OPEN: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/FolderOpen.ico")).into();
#[dynamic]
pub static LOGS: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Logs.ico")).into();
#[dynamic]
pub static REFRESH: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Refresh.ico")).into();
#[dynamic]
pub static RELOAD: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Reload.ico")).into();
#[dynamic]
pub static SAVE: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Save.ico")).into();
#[dynamic]
pub static SETTINGS: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Settings.ico")).into();
#[dynamic]
pub static START: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Start.ico")).into();
#[dynamic]
pub static STOP: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Stop.ico")).into();
#[dynamic]
pub static UP: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Up.ico")).into();
#[dynamic]
pub static VALIDATE: image::Handle =
    image::Handle::from_memory(std::include_bytes!("../res/icons/Validate.ico")).into();
