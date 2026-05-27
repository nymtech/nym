use tauri::menu::{Menu, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Runtime};

use crate::platform_constants::SECONDARY_LOG_WEBVIEW_SUPPORTED;

pub const SHOW_LOG_WINDOW: &str = "show_log_window";

pub fn build_app_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let edit_submenu = SubmenuBuilder::new(app, "Edit")
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    let mut menu_builder = MenuBuilder::new(app).item(&edit_submenu);

    if std::env::var("NYM_WALLET_ENABLE_LOG").is_ok() && SECONDARY_LOG_WEBVIEW_SUPPORTED {
        let help_text = MenuItemBuilder::with_id(SHOW_LOG_WINDOW, "Show logs").build(app)?;

        let help_submenu = SubmenuBuilder::new(app, "Help")
            .items(&[&help_text])
            .build()?;

        menu_builder = menu_builder.item(&help_submenu);
    }

    menu_builder.build()
}
