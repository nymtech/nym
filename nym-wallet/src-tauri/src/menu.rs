use tauri::Menu;
use tauri::{CustomMenuItem, Submenu};

pub const SHOW_LOG_WINDOW: &str = "show_log_window";

pub trait AddDefaultSubmenus {
    fn add_default_app_submenus(self) -> Self;
}

impl AddDefaultSubmenus for Menu {
    #[allow(dead_code)]
    fn add_default_app_submenus(self) -> Self {
        if ::std::env::var("NYM_WALLET_ENABLE_LOG").is_ok() {
            let submenu = Submenu::new(
                "Help",
                Menu::new().add_item(CustomMenuItem::new(SHOW_LOG_WINDOW, "Show logs")),
            );
            return self.add_submenu(submenu);
        }
        self
    }
}
