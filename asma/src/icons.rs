use iced::widget::image;
use static_init::dynamic;

#[dynamic]
pub static CANCEL: image::Handle = image::Handle::from_memory(std::include_bytes!("../res/icons/Cancel.ico")).into();
#[dynamic]
pub static FOLDER_OPEN: image::Handle = image::Handle::from_memory(std::include_bytes!("../res/icons/FolderOpen.ico")).into();
#[dynamic]
pub static REFRESH: image::Handle = image::Handle::from_memory(std::include_bytes!("../res/icons/Refresh.ico")).into();
#[dynamic]
pub static SETTINGS: image::Handle = image::Handle::from_memory(std::include_bytes!("../res/icons/Settings.ico")).into();

