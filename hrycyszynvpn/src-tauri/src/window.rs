use crate::menu::TRAY_MENU_SHOW_HIDE;
use tauri::{AppHandle, Manager};

pub(crate) fn window_hide(app: &AppHandle<tauri::Wry>) {
  let window = app.get_window("main").unwrap();
  let item_handle = app.tray_handle().get_item(TRAY_MENU_SHOW_HIDE);
  if window.is_visible().unwrap() {
    window.hide().unwrap();
    item_handle.set_title("Show").unwrap();
  }
}

pub(crate) fn window_show(app: &AppHandle<tauri::Wry>) {
  let window = app.get_window("main").unwrap();
  let item_handle = app.tray_handle().get_item(TRAY_MENU_SHOW_HIDE);
  if !window.is_visible().unwrap() {
    window.show().unwrap();
    item_handle.set_title("Hide").unwrap();
    window.set_focus().unwrap();
  }
}

pub(crate) fn window_toggle(app: &AppHandle<tauri::Wry>) {
  let window = app.get_window("main").unwrap();
  if window.is_visible().unwrap() {
    window_hide(app);
  } else {
    window_show(app);
  }
}
