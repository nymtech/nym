use crate::window_toggle;
use tauri::{
    AppHandle, CustomMenuItem, Menu, SystemTray, SystemTrayEvent, SystemTrayMenu,
    SystemTrayMenuItem, Wry,
};
#[cfg(target_os = "macos")]
use tauri::{MenuItem, Submenu};

pub trait AddDefaultSubmenus {
    fn add_default_app_submenu_if_macos(self) -> Self;
}

impl AddDefaultSubmenus for Menu {
    fn add_default_app_submenu_if_macos(self) -> Menu {
        #[cfg(target_os = "macos")]
        return self
            .add_submenu(Submenu::new(
                "File",
                Menu::new().add_native_item(MenuItem::Quit),
            ))
            .add_submenu(Submenu::new(
                "Edit",
                Menu::new()
                    .add_native_item(MenuItem::Copy)
                    .add_native_item(MenuItem::Cut)
                    .add_native_item(MenuItem::Paste)
                    .add_native_item(MenuItem::SelectAll),
            ))
            .add_submenu(Submenu::new(
                "Window",
                Menu::new()
                    .add_native_item(MenuItem::Hide)
                    .add_native_item(MenuItem::HideOthers)
                    .add_native_item(MenuItem::ShowAll),
            ));
        #[cfg(not(target_os = "macos"))]
        return self;
    }
}

pub const TRAY_MENU_QUIT: &str = "quit";
pub const TRAY_MENU_SHOW_HIDE: &str = "show-hide";
pub const TRAY_MENU_CONNECTION: &str = "connection";

pub(crate) fn create_tray_menu() -> SystemTray {
    let quit = CustomMenuItem::new(TRAY_MENU_QUIT, "Quit");
    let hide = CustomMenuItem::new(TRAY_MENU_SHOW_HIDE, "Hide");
    let connection = CustomMenuItem::new(TRAY_MENU_CONNECTION, "Connect");
    let tray_menu = SystemTrayMenu::new()
        .add_item(hide)
        .add_item(connection)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);

    SystemTray::new().with_menu(tray_menu)
}

pub(crate) fn tray_menu_event_handler(app: &AppHandle<Wry>, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::LeftClick { position, size, .. } => {
            println!("Event {:?} {:?}", position, size);
        }
        SystemTrayEvent::MenuItemClick { id, .. } => {
            println!("Event {}", id);
            match id.as_str() {
                TRAY_MENU_SHOW_HIDE => {
                    window_toggle(app);
                }
                TRAY_MENU_QUIT => {
                    // TODO: add disconnecting first
                    app.exit(0);
                }
                _ => {}
            }
        }
        _ => {}
    }
}
