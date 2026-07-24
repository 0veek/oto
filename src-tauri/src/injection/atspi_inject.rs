//! AT-SPI2 text insertion into the focused accessible object.

use std::collections::{HashSet, VecDeque};
use std::time::{Duration, Instant};

use atspi::proxy::accessible::AccessibleProxy;
use atspi::proxy::proxy_ext::ProxyExt;
use atspi::{AccessibilityConnection, Interface, ObjectRefOwned, State};

use crate::error::{OtoError, OtoResult};

const AT_SPI_BUDGET: Duration = Duration::from_millis(400);
const AT_SPI_MAX_NODES: usize = 2_500;

async fn accessible_for<'a>(
    connection: &'a AccessibilityConnection,
    object: &ObjectRefOwned,
) -> OtoResult<AccessibleProxy<'a>> {
    let destination = object
        .name()
        .ok_or_else(|| OtoError::Message("AT-SPI object has no bus name".into()))?
        .clone();
    AccessibleProxy::builder(connection.connection())
        .destination(destination)
        .map_err(|error| OtoError::Message(format!("AT-SPI destination: {error}")))?
        .path(object.path().clone())
        .map_err(|error| OtoError::Message(format!("AT-SPI path: {error}")))?
        .build()
        .await
        .map_err(|error| OtoError::Message(format!("AT-SPI proxy: {error}")))
}

fn object_key(object: &ObjectRefOwned) -> (String, String) {
    let name = object
        .name()
        .map(|n| n.to_string())
        .unwrap_or_default();
    let path = object.path().to_string();
    (name, path)
}

async fn insert_into_focused(text: &str) -> OtoResult<bool> {
    let deadline = Instant::now() + AT_SPI_BUDGET;
    let connection = AccessibilityConnection::new()
        .await
        .map_err(|error| OtoError::Message(format!("AT-SPI unavailable: {error}")))?;
    let root = connection
        .root_accessible_on_registry()
        .await
        .map_err(|error| OtoError::Message(format!("AT-SPI root: {error}")))?;
    let children = root
        .get_children()
        .await
        .map_err(|error| OtoError::Message(format!("AT-SPI applications: {error}")))?;

    // Prefer active/focused applications so we find the caret before the budget expires.
    let mut prioritized = Vec::new();
    let mut rest = Vec::new();
    for child in children {
        if child.is_null() {
            continue;
        }
        let is_active = match accessible_for(&connection, &child).await {
            Ok(accessible) => accessible
                .get_state()
                .await
                .ok()
                .is_some_and(|state| state.contains(State::Active) || state.contains(State::Focused)),
            Err(_) => false,
        };
        if is_active {
            prioritized.push(child);
        } else {
            rest.push(child);
        }
    }

    let mut queue: VecDeque<ObjectRefOwned> = prioritized.into_iter().chain(rest).collect();
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut visited = 0usize;

    while let Some(object) = queue.pop_front() {
        if Instant::now() >= deadline || visited >= AT_SPI_MAX_NODES {
            break;
        }
        // Always count dequeued nodes so nulls cannot spin the loop forever.
        visited += 1;
        if object.is_null() {
            continue;
        }
        if !seen.insert(object_key(&object)) {
            continue;
        }
        let accessible = match accessible_for(&connection, &object).await {
            Ok(accessible) => accessible,
            Err(_) => continue,
        };
        let state = accessible.get_state().await.ok();
        let interfaces = accessible.get_interfaces().await.ok();
        let focused_editable = state
            .is_some_and(|state| state.contains(State::Focused) && state.contains(State::Editable))
            && interfaces.is_some_and(|interfaces| {
                interfaces.contains(Interface::EditableText) && interfaces.contains(Interface::Text)
            });

        if focused_editable {
            let proxies = accessible
                .proxies()
                .await
                .map_err(|error| OtoError::Message(format!("AT-SPI interfaces: {error}")))?;
            let editable = proxies
                .editable_text()
                .await
                .map_err(|error| OtoError::Message(format!("AT-SPI editable text: {error}")))?;
            let text_proxy = proxies
                .text()
                .await
                .map_err(|error| OtoError::Message(format!("AT-SPI text: {error}")))?;
            let position = if text_proxy.get_n_selections().await.unwrap_or(0) > 0 {
                let (start, end) = text_proxy
                    .get_selection(0)
                    .await
                    .map_err(|error| OtoError::Message(format!("AT-SPI selection: {error}")))?;
                let selection_start = start.min(end);
                let selection_end = start.max(end);
                if selection_start != selection_end
                    && !editable
                        .delete_text(selection_start, selection_end)
                        .await
                        .unwrap_or(false)
                {
                    return Ok(false);
                }
                selection_start
            } else {
                text_proxy.caret_offset().await.unwrap_or(0)
            };
            // AT-SPI InsertText `length` is the UTF-8 byte length of `text`,
            // not the Unicode scalar count (non-ASCII would otherwise truncate).
            return editable
                .insert_text(position, text, text.len() as i32)
                .await
                .map_err(|error| OtoError::Message(format!("AT-SPI insert: {error}")));
        }

        if Instant::now() >= deadline {
            break;
        }
        if let Ok(children) = accessible.get_children().await {
            for child in children {
                if !child.is_null() && !seen.contains(&object_key(&child)) {
                    queue.push_back(child);
                }
            }
        }
    }
    Ok(false)
}

async fn selection_from_focused() -> OtoResult<Option<String>> {
    let deadline = Instant::now() + AT_SPI_BUDGET;
    let connection = AccessibilityConnection::new()
        .await
        .map_err(|error| OtoError::Message(format!("AT-SPI unavailable: {error}")))?;
    let root = connection
        .root_accessible_on_registry()
        .await
        .map_err(|error| OtoError::Message(format!("AT-SPI root: {error}")))?;
    let mut queue: VecDeque<ObjectRefOwned> = root
        .get_children()
        .await
        .map_err(|error| OtoError::Message(format!("AT-SPI applications: {error}")))?
        .into();
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut visited = 0usize;
    while let Some(object) = queue.pop_front() {
        if Instant::now() >= deadline || visited >= AT_SPI_MAX_NODES {
            break;
        }
        visited += 1;
        if object.is_null() {
            continue;
        }
        if !seen.insert(object_key(&object)) {
            continue;
        }
        let accessible = match accessible_for(&connection, &object).await {
            Ok(accessible) => accessible,
            Err(_) => continue,
        };
        let focused_text = accessible
            .get_state()
            .await
            .ok()
            .is_some_and(|state| state.contains(State::Focused))
            && accessible
                .get_interfaces()
                .await
                .ok()
                .is_some_and(|interfaces| interfaces.contains(Interface::Text));
        if focused_text {
            let proxies = accessible
                .proxies()
                .await
                .map_err(|error| OtoError::Message(format!("AT-SPI interfaces: {error}")))?;
            let text_proxy = proxies
                .text()
                .await
                .map_err(|error| OtoError::Message(format!("AT-SPI text: {error}")))?;
            if text_proxy.get_n_selections().await.unwrap_or(0) > 0 {
                let (start, end) = text_proxy
                    .get_selection(0)
                    .await
                    .map_err(|error| OtoError::Message(format!("AT-SPI selection: {error}")))?;
                let value = text_proxy
                    .get_text(start.min(end), start.max(end))
                    .await
                    .map_err(|error| OtoError::Message(format!("AT-SPI selected text: {error}")))?;
                if !value.trim().is_empty() {
                    return Ok(Some(value));
                }
            }
            return Ok(None);
        }
        if Instant::now() >= deadline {
            break;
        }
        if let Ok(children) = accessible.get_children().await {
            for child in children {
                if !child.is_null() && !seen.contains(&object_key(&child)) {
                    queue.push_back(child);
                }
            }
        }
    }
    Ok(None)
}

/// Try to insert `text` via AT-SPI2 into the focused accessible text widget.
///
/// Returns `Ok(true)` if insertion succeeded, `Ok(false)` if AT-SPI is
/// unavailable or unsupported (caller should try the next injection method).
pub async fn try_atspi_insert(text: &str) -> OtoResult<bool> {
    match tokio::time::timeout(
        Duration::from_millis(450),
        insert_into_focused(text),
    )
    .await
    {
        Ok(Ok(inserted)) => Ok(inserted),
        Ok(Err(error)) => {
            eprintln!("oto injection: {error}");
            Ok(false)
        }
        Err(_) => {
            eprintln!("oto injection: AT-SPI lookup timed out");
            Ok(false)
        }
    }
}

pub async fn try_atspi_selection() -> OtoResult<Option<String>> {
    match tokio::time::timeout(
        Duration::from_millis(450),
        selection_from_focused(),
    )
    .await
    {
        Ok(Ok(selection)) => Ok(selection),
        Ok(Err(error)) => {
            eprintln!("oto command mode: {error}");
            Ok(None)
        }
        Err(_) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn headless_lookup_never_panics() {
        let _ = try_atspi_insert("hello").await.unwrap();
    }
}
