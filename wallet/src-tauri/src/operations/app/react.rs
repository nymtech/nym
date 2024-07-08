use crate::error::BackendError;
use crate::state::WalletState;

#[tauri::command]
pub async fn get_react_state(
    state: tauri::State<'_, WalletState>,
) -> Result<Option<String>, BackendError> {
    let r_state = state.read().await;
    r_state.get_react_state()
}

#[tauri::command]
pub async fn set_react_state(
    state: tauri::State<'_, WalletState>,
    new_state: Option<String>,
) -> Result<(), BackendError> {
    let mut w_state = state.write().await;
    w_state.set_react_state(new_state)
}
