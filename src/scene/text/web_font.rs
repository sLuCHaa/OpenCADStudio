//! Per-script web font store + lazy fetch (#141).
//!
//! The desktop build outlines glyphs for non-LFF scripts from the user's
//! installed system fonts (see `ttf_glyph::build_fallback`). The web build has
//! no system-font access, so it instead lazily fetches a per-script Noto subset
//! — served from `fonts/<script>.ttf`, one alphabet per file — the first time a
//! drawing uses that script, then outlines glyphs from it. Splitting per script
//! keeps each download small (Latin/Cyrillic/Greek ~50–100 KB; CJK loads only
//! when a CJK drawing is opened).
//!
//! The store and fetch are web-only; the desktop side keeps no-op stubs so the
//! shared call sites (`ttf_glyph`, the app message loop) compile unchanged.

use std::sync::atomic::{AtomicU8, Ordering};

/// A script we ship a Noto subset for. [`script_of`] maps a char to one.
///
/// CJK is split by language — Chinese, Japanese and Korean each get their own
/// file. Their ideographs (Han, U+4E00–9FFF) share the same code points but
/// differ in glyph shape, so the shared block is routed by the document's
/// language (see [`set_cjk_lang_from_codepage`]); kana is always Japanese and
/// Hangul always Korean.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Script {
    Latin,
    Cyrillic,
    Greek,
    Arabic,
    Hebrew,
    Thai,
    Devanagari,
    Chinese,
    Japanese,
    Korean,
}

impl Script {
    /// Same-origin asset path the web build fetches this script's font from.
    /// Must match the files produced by `web/fonts/generate.sh`.
    pub fn asset(self) -> &'static str {
        match self {
            Script::Latin => "fonts/latin.ttf",
            Script::Cyrillic => "fonts/cyrillic.ttf",
            Script::Greek => "fonts/greek.ttf",
            Script::Arabic => "fonts/arabic.ttf",
            Script::Hebrew => "fonts/hebrew.ttf",
            Script::Thai => "fonts/thai.ttf",
            Script::Devanagari => "fonts/devanagari.ttf",
            Script::Chinese => "fonts/chinese.ttf",
            Script::Japanese => "fonts/japanese.ttf",
            Script::Korean => "fonts/korean.ttf",
        }
    }
}

/// Language used to render shared Han ideographs: 0 = Chinese, 1 = Japanese,
/// 2 = Korean. Set from the active document's code page.
static CJK_LANG: AtomicU8 = AtomicU8::new(0);

fn cjk_lang() -> Script {
    match CJK_LANG.load(Ordering::Relaxed) {
        1 => Script::Japanese,
        2 => Script::Korean,
        _ => Script::Chinese,
    }
}

/// Point the shared-Han routing at a language based on a DWG/DXF code page
/// (`$DWGCODEPAGE`), e.g. `ANSI_932` → Japanese, `ANSI_949` → Korean, GB/936 or
/// anything else → Chinese. Returns `true` if the language changed (the caller
/// then clears the glyph cache and re-tessellates).
pub fn set_cjk_lang_from_codepage(code_page: &str) -> bool {
    let c = code_page.to_ascii_uppercase();
    let lang = if c.contains("932") || c.contains("SJIS") || c.contains("SHIFT") {
        1 // Japanese (Shift-JIS)
    } else if c.contains("949") || c.contains("KOR") || c.contains("UHC") {
        2 // Korean
    } else {
        0 // Chinese (936 / GB / 950 / Big5) or non-CJK default
    };
    CJK_LANG.swap(lang, Ordering::Relaxed) != lang
}

/// The script font that covers `ch`, or `None` for control / uncovered code
/// points. Ranges mirror the subset unicode ranges in `web/fonts/generate.sh`.
pub fn script_of(ch: char) -> Option<Script> {
    Some(match ch as u32 {
        0x0000..=0x024F | 0x1E00..=0x1EFF | 0x2000..=0x206F => Script::Latin,
        0x0370..=0x03FF | 0x1F00..=0x1FFF => Script::Greek,
        0x0400..=0x052F | 0x2DE0..=0x2DFF | 0xA640..=0xA69F => Script::Cyrillic,
        0x0590..=0x05FF | 0xFB1D..=0xFB4F => Script::Hebrew,
        0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF => {
            Script::Arabic
        }
        0x0900..=0x097F => Script::Devanagari,
        // Hangul → always Korean; kana → always Japanese.
        0x1100..=0x11FF | 0x3130..=0x318F | 0xAC00..=0xD7A3 => Script::Korean,
        0x3040..=0x30FF | 0x31F0..=0x31FF => Script::Japanese,
        // Shared Han + CJK symbols + fullwidth → routed by the document language.
        0x3000..=0x303F | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0xFF00..=0xFFEF => {
            cjk_lang()
        }
        _ => return None,
    })
}

// ── Web store ───────────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
mod imp {
    use super::Script;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex, OnceLock};

    enum State {
        Loading,
        Loaded(Arc<Vec<u8>>),
        Failed,
    }

    #[derive(Default)]
    struct Store {
        states: HashMap<Script, State>,
        /// Scripts requested but not yet fetched; the app drains this and kicks
        /// off the fetch tasks.
        pending: Vec<Script>,
    }

    fn store() -> &'static Mutex<Store> {
        static S: OnceLock<Mutex<Store>> = OnceLock::new();
        S.get_or_init(|| Mutex::new(Store::default()))
    }

    /// Loaded font bytes for `script`, or `None` — queueing a fetch the first
    /// time a script is missed so the app loop can load it.
    pub fn request(script: Script) -> Option<Arc<Vec<u8>>> {
        let mut s = store().lock().unwrap();
        match s.states.get(&script) {
            Some(State::Loaded(b)) => Some(b.clone()),
            Some(_) => None, // Loading or Failed — don't re-queue.
            None => {
                s.states.insert(script, State::Loading);
                s.pending.push(script);
                None
            }
        }
    }

    /// Drain the scripts awaiting a fetch.
    pub fn take_pending() -> Vec<Script> {
        std::mem::take(&mut store().lock().unwrap().pending)
    }

    /// Record a fetch result: `Some(bytes)` on success, `None` on failure.
    pub fn insert(script: Script, bytes: Option<Vec<u8>>) {
        let st = match bytes {
            Some(b) => State::Loaded(Arc::new(b)),
            None => State::Failed,
        };
        store().lock().unwrap().states.insert(script, st);
    }

    /// Fetch a script font over HTTP from the same origin.
    pub async fn fetch(script: Script) -> Result<Vec<u8>, String> {
        use wasm_bindgen::JsCast;
        use wasm_bindgen_futures::JsFuture;
        let win = web_sys::window().ok_or("no window")?;
        let resp_val = JsFuture::from(win.fetch_with_str(script.asset()))
            .await
            .map_err(|e| format!("{e:?}"))?;
        let resp: web_sys::Response = resp_val.dyn_into().map_err(|_| "bad response".to_string())?;
        if !resp.ok() {
            return Err(format!("HTTP {}", resp.status()));
        }
        let ab = JsFuture::from(resp.array_buffer().map_err(|e| format!("{e:?}"))?)
            .await
            .map_err(|e| format!("{e:?}"))?;
        Ok(js_sys::Uint8Array::new(&ab).to_vec())
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use super::Script;
    use std::sync::Arc;

    pub fn request(_script: Script) -> Option<Arc<Vec<u8>>> {
        None
    }
    pub fn take_pending() -> Vec<Script> {
        Vec::new()
    }
    pub fn insert(_script: Script, _bytes: Option<Vec<u8>>) {}
    pub async fn fetch(_script: Script) -> Result<Vec<u8>, String> {
        Err("web only".into())
    }
}

pub use imp::{fetch, insert, request, take_pending};
