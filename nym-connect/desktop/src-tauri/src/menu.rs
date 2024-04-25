use tauri::{
    AppHandle, CustomMenuItem, Menu, Submenu, SystemTray, SystemTrayEvent, SystemTrayMenu,
    SystemTrayMenuItem, Wry,
};

use crate::window_toggle;

pub const SHOW_LOG_WINDOW: &str = "show_log_window";
pub const CLEAR_STORAGE: &str = "clear_storage";

pub trait AddDefaultSubmenus {
    #[allow(dead_code)]
    fn add_default_app_submenus(self) -> Self;
}

impl AddDefaultSubmenus for Menu {
    fn add_default_app_submenus(self) -> Self {
        let submenu = Submenu::new(
            "Help",
            Menu::new().add_item(CustomMenuItem::new(SHOW_LOG_WINDOW, "Show logs")),
        );
        self.add_submenu(submenu)
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
            println!("Event {position:?} {size:?}");
        }
        SystemTrayEvent::MenuItemClick { id, .. } => {
            println!("Event {id}");
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
