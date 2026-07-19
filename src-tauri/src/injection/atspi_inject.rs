//! AT-SPI2 text insertion into the focused accessible object.
//!
//! MVP stub: full AT-SPI (zbus + focused text interface) is deferred.
//! Returning `Ok(false)` lets the hybrid chain fall through to clipboard+paste.
//! Follow-up: implement focused EditableText insert via atspi/zbus when feasible.

use crate::error::OtoResult;

/// Try to insert `text` via AT-SPI2 into the focused accessible text widget.
///
/// Returns `Ok(true)` if insertion succeeded, `Ok(false)` if AT-SPI is
/// unavailable or unsupported (caller should try the next injection method).
pub async fn try_atspi_insert(text: &str) -> OtoResult<bool> {
    // Intentionally unimplemented for MVP — hybrid clipboard chain still covers
    // the product requirement. Do not error here; false triggers fallback.
    let _ = text;
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stub_returns_false() {
        assert_eq!(try_atspi_insert("hello").await.unwrap(), false);
    }
}
