use std::env;

#[tauri::command]
pub async fn get_env(variable: String) -> Option<String> {
    let var = env::var(&variable).ok();
    log::trace!("get_env {variable} {:?}", var);

    var
}
