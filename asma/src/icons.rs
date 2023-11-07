use iced::widget::image;
use static_init::dynamic;

#[dynamic]
pub static ADD: image::Handle = image::Handle::from_memory(std::include_bytes!("../res/icons/Add.ico")).into();
#[dynamic]
pub static CANCEL: image::Handle = image::Handle::from_memory(std::include_bytes!("../res/icons/Cancel.ico")).into();
#[dynamic]
pub static DOWNLOAD: image::Handle = image::Handle::from_memory(std::include_bytes!("../res/icons/Download.ico")).into();
#[dynamic]
pub static EDIT: image::Handle = image::Handle::from_memory(std::include_bytes!("../res/icons/Edit.ico")).into();
#[dynamic]
pub static FOLDER_OPEN: image::Handle = image::Handle::from_memory(std::include_bytes!("../res/icons/FolderOpen.ico")).into();
#[dynamic]
pub static REFRESH: image::Handle = image::Handle::from_memory(std::include_bytes!("../res/icons/Refresh.ico")).into();
#[dynamic]
pub static SAVE: image::Handle = image::Handle::from_memory(std::include_bytes!("../res/icons/Save.ico")).into();
#[dynamic]
pub static SETTINGS: image::Handle = image::Handle::from_memory(std::include_bytes!("../res/icons/Settings.ico")).into();
