use crate::error::OtoError;
use crate::features::history::{self, HistoryEntry};
use crate::injection::set_clipboard_text;

#[tauri::command]
pub fn get_history() -> Result<Vec<HistoryEntry>, OtoError> {
    history::list()
}

#[tauri::command]
pub fn delete_history_entry(id: String) -> Result<(), OtoError> {
    history::delete(&id)
}

#[tauri::command]
pub fn clear_history() -> Result<(), OtoError> {
    history::clear()
}

#[tauri::command]
pub fn copy_history_text(text: String) -> Result<(), OtoError> {
    set_clipboard_text(&text)
}
