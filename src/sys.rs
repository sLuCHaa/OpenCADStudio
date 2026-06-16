// Small platform shims for things the desktop build does natively but the web
// (wasm) build must handle differently or skip.

/// Open a URL in the user's browser. The desktop launches the default handler;
/// the web opens a new tab (the button click is a user gesture, so it isn't
/// caught by the pop-up blocker). Focus of the opened page is left to the
/// OS / browser.
#[cfg(not(target_arch = "wasm32"))]
pub fn open_url(url: &str) {
    let _ = open::that(url);
}

#[cfg(target_arch = "wasm32")]
pub fn open_url(url: &str) {
    if let Some(window) = web_sys::window() {
        let _ = window.open_with_url_and_target(url, "_blank");
    }
}

/// Turn an `rfd` file handle into a `PathBuf` the rest of the app keys on.
///
/// Desktop returns the real filesystem path. The browser has no path, so we
/// synthesize one from the file name — enough for the app to compile and track
/// the document name; actual byte I/O on the web reads the handle directly
/// (a follow-up).
#[cfg(not(target_arch = "wasm32"))]
pub fn handle_path(h: &rfd::FileHandle) -> std::path::PathBuf {
    h.path().to_path_buf()
}

#[cfg(target_arch = "wasm32")]
pub fn handle_path(h: &rfd::FileHandle) -> std::path::PathBuf {
    std::path::PathBuf::from(h.file_name())
}

/// Trigger a browser download of `bytes` as `name`. Builds a Blob, points a
/// hidden `<a download>` at it and clicks it programmatically — because this
/// runs inside the Save button's click (a user gesture), the file downloads
/// immediately with no extra "click to download" link. Web only.
#[cfg(target_arch = "wasm32")]
pub fn download_bytes(name: &str, bytes: &[u8]) {
    use wasm_bindgen::JsCast;
    let Some(window) = web_sys::window() else { return };
    let Some(document) = window.document() else { return };
    let array = js_sys::Uint8Array::from(bytes);
    let parts = js_sys::Array::new();
    parts.push(&array.buffer());
    let Ok(blob) = web_sys::Blob::new_with_u8_array_sequence(&parts) else {
        return;
    };
    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
        return;
    };
    if let Ok(el) = document.create_element("a") {
        let a: web_sys::HtmlAnchorElement = el.unchecked_into();
        a.set_href(&url);
        a.set_download(name);
        a.click();
    }
    let _ = web_sys::Url::revoke_object_url(&url);
}
