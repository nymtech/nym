use tauri::menu::Menu;
use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};

pub const SHOW_LOG_WINDOW: &str = "show_log_window";

pub trait AddDefaultSubmenus {
    #[allow(dead_code)]
    fn add_default_app_submenus(self) -> Self;
}

impl<R: tauri::Runtime> AddDefaultSubmenus for Menu<R> {
    #[allow(dead_code)]
    fn add_default_app_submenus(self) -> Self {
        if ::std::env::var("NYM_WALLET_ENABLE_LOG").is_ok() {
            let app_handle = self.app_handle();

            let help_text = MenuItemBuilder::with_id(SHOW_LOG_WINDOW, "Show logs")
                .build(app_handle)
                .expect("Failed to create menu item");

            let submenu = SubmenuBuilder::new(app_handle, "Help")
                .items(&[&help_text])
                .build()
                .expect("Failed to create help submenu");

            let menu_builder = MenuBuilder::new(app_handle);

            match menu_builder.item(&submenu).build() {
                Ok(new_menu) => new_menu,
                Err(_) => self,
            }
        } else {
            self
        }
    }
}
