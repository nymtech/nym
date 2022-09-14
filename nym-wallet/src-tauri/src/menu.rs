use tauri::Menu;
use tauri::{CustomMenuItem, Submenu};

pub const SHOW_LOG_WINDOW: &str = "show_log_window";

pub trait AddDefaultSubmenus {
    fn add_default_app_submenus(self) -> Self;
}

impl AddDefaultSubmenus for Menu {
    fn add_default_app_submenus(self) -> Menu {
        if ::std::env::var("NYM_WALLET_ENABLE_MENUBAR").is_ok() {
            let submenu = Submenu::new(
                "Help",
                Menu::new().add_item(CustomMenuItem::new(SHOW_LOG_WINDOW, "Show logs")),
            );
            self.add_submenu(submenu)
        } else {
            // This is the old behaviour
            // Remove this branch once we're happy with the menubar behaviour
            #[cfg(target_os = "macos")]
            return self.add_submenu(Submenu::new(
                "Menu",
                Menu::new()
                    .add_native_item(MenuItem::Copy)
                    .add_native_item(MenuItem::Cut)
                    .add_native_item(MenuItem::Paste)
                    .add_native_item(MenuItem::Hide)
                    .add_native_item(MenuItem::HideOthers)
                    .add_native_item(MenuItem::SelectAll)
                    .add_native_item(MenuItem::ShowAll)
                    .add_native_item(MenuItem::Quit),
            ));
            #[cfg(not(target_os = "macos"))]
            return self;
        }
    }
}
