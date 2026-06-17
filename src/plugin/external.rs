//! Phase-2 external plugin discovery.
//!
//! Scans the per-user plugins directory for installed add-on packages and
//! reads their `plugin.toml` so the host can list them and gate them on the
//! API version — *before* any native code is loaded. Actually loading the
//! `cdylib` is a separate step; this module only inspects what is on disk.
//!
//! Layout (mirrors the spec in `docs/plugin-architecture.md`):
//! ```text
//! <config>/OpenCADStudio/plugins/
//!   opencad.storm_sewer/
//!     plugin.toml
//!     <libopencad_storm_sewer.so | .dll | .dylib>
//! ```

use std::path::PathBuf;

/// An add-on package found on disk (not necessarily loaded or compatible).
#[derive(Debug, Clone)]
pub struct ExternalPlugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub api_version: u32,
    pub ribbon_order: i32,
    pub command_prefixes: Vec<String>,
    /// The package directory under the plugins folder.
    pub dir: PathBuf,
    /// Whether a native library for this platform sits beside `plugin.toml`.
    pub lib_present: bool,
}

impl ExternalPlugin {
    /// True when the package's API version matches the host ABI major.
    pub fn api_compatible(&self) -> bool {
        self.api_version == ocs_plugin_api::API_VERSION
    }

    /// True when the package can be loaded today: compatible API *and* a native
    /// library present for this platform.
    pub fn loadable(&self) -> bool {
        self.api_compatible() && self.lib_present
    }
}

/// `<config>/OpenCADStudio/plugins`, matching the settings/recent-files store.
pub fn plugins_dir() -> Option<PathBuf> {
    let base: PathBuf = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from)?
    } else if cfg!(target_os = "macos") {
        let home = std::env::var_os("HOME")?;
        let mut p = PathBuf::from(home);
        p.push("Library");
        p.push("Application Support");
        p
    } else if let Some(d) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(d)
    } else {
        let home = std::env::var_os("HOME")?;
        let mut p = PathBuf::from(home);
        p.push(".config");
        p
    };
    let mut p = base;
    p.push("OpenCADStudio");
    p.push("plugins");
    Some(p)
}

/// Native dynamic-library extension for the current platform (no dot).
fn lib_extension() -> &'static str {
    if cfg!(target_os = "windows") {
        "dll"
    } else if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    }
}

/// Discover every package under the plugins directory, sorted by `ribbon_order`
/// then id. Missing directory → empty list (not an error).
pub fn discover() -> Vec<ExternalPlugin> {
    let Some(root) = plugins_dir() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(&root) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let toml_path = dir.join("plugin.toml");
        let Ok(text) = std::fs::read_to_string(&toml_path) else {
            continue;
        };
        if let Some(mut p) = parse_plugin_toml(&text) {
            p.lib_present = lib_present_in(&dir);
            p.dir = dir;
            found.push(p);
        }
    }
    found.sort_by(|a, b| a.ribbon_order.cmp(&b.ribbon_order).then(a.id.cmp(&b.id)));
    found
}

/// True when a file with this platform's dynamic-library extension exists in
/// `dir` (any name — the package owns its lib naming).
fn lib_present_in(dir: &std::path::Path) -> bool {
    let ext = lib_extension();
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .any(|e| e.path().extension().and_then(|s| s.to_str()) == Some(ext))
        })
        .unwrap_or(false)
}

/// Minimal `plugin.toml` reader for the documented `[plugin]` / `[opencad]`
/// keys. Deliberately small (string / integer / string-array values) so the
/// host doesn't pull in a full TOML parser for a fixed, host-defined schema.
/// Returns `None` when the required `id` is missing. `dir` / `lib_present` are
/// filled in by the caller.
fn parse_plugin_toml(text: &str) -> Option<ExternalPlugin> {
    let mut id = None;
    let mut name = String::new();
    let mut version = String::new();
    let mut description = String::new();
    let mut api_version: u32 = 0;
    let mut ribbon_order: i32 = 0;
    let mut command_prefixes: Vec<String> = Vec::new();

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "id" => id = Some(unquote(value)),
            "name" => name = unquote(value),
            "version" => version = unquote(value),
            "description" => description = unquote(value),
            "api_version" => api_version = value.parse().unwrap_or(0),
            "ribbon_order" => ribbon_order = value.parse().unwrap_or(0),
            "command_prefixes" => command_prefixes = parse_string_array(value),
            _ => {}
        }
    }

    Some(ExternalPlugin {
        id: id?,
        name,
        version,
        description,
        api_version,
        ribbon_order,
        command_prefixes,
        dir: PathBuf::new(),
        lib_present: false,
    })
}

/// Strip surrounding single or double quotes from a TOML scalar.
fn unquote(s: &str) -> String {
    let s = s.trim();
    let bytes = s.as_bytes();
    if bytes.len() >= 2
        && (bytes[0] == b'"' || bytes[0] == b'\'')
        && bytes[bytes.len() - 1] == bytes[0]
    {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

/// Parse `["a", "b"]` into `["a", "b"]`. Tolerant of spacing and missing
/// brackets; ignores empty entries.
fn parse_string_array(s: &str) -> Vec<String> {
    s.trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(unquote)
        .filter(|e| !e.is_empty())
        .collect()
}

// ── Runtime loading (desktop only) ──────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
pub use loader::{load, load_at_startup, loaded_ids, with_loaded, LoadedPlugin};

#[cfg(not(target_arch = "wasm32"))]
mod loader {
    use super::{lib_extension, ExternalPlugin};
    use ocs_plugin_api::host::BuiltinPlugin;
    use std::path::{Path, PathBuf};

    /// A loaded external plugin. The library must outlive the boxed plugin, so
    /// `plugin` is declared before `_lib` (fields drop in declaration order).
    pub struct LoadedPlugin {
        plugin: Box<dyn BuiltinPlugin>,
        _lib: libloading::Library,
        pub id: String,
    }

    impl LoadedPlugin {
        pub fn plugin(&self) -> &dyn BuiltinPlugin {
            self.plugin.as_ref()
        }
    }

    use std::cell::RefCell;

    // Process-wide store of loaded external plugins. The libraries must stay
    // resident for the whole session — ribbon tabs and command dispatch hold
    // vtables that live inside them — so this is filled once at startup and
    // never cleared mid-session (reloading would dangle live ribbon modules).
    thread_local! {
        static LOADED: RefCell<Vec<LoadedPlugin>> = const { RefCell::new(Vec::new()) };
    }

    /// Discover packages and load every API-compatible one with a native
    /// library into the process store. Call once at startup. Returns per-id
    /// results so the host can report load failures.
    pub fn load_at_startup() -> Vec<(String, Result<(), String>)> {
        let discovered = super::discover();
        let mut out = Vec::new();
        LOADED.with(|cell| {
            let mut store = cell.borrow_mut();
            if !store.is_empty() {
                return; // already loaded this session
            }
            for d in &discovered {
                if !d.api_compatible() || !d.lib_present {
                    continue;
                }
                match load(d) {
                    Ok(lp) => {
                        out.push((lp.id.clone(), Ok(())));
                        store.push(lp);
                    }
                    Err(e) => out.push((d.id.clone(), Err(e))),
                }
            }
        });
        out
    }

    /// Ids of the plugins currently loaded in the process store.
    pub fn loaded_ids() -> Vec<String> {
        LOADED.with(|c| c.borrow().iter().map(|lp| lp.id.clone()).collect())
    }

    /// Run `f` over the loaded plugins (borrowing the store).
    pub fn with_loaded<R>(f: impl FnOnce(&[LoadedPlugin]) -> R) -> R {
        LOADED.with(|c| f(&c.borrow()))
    }

    /// Path to the native library beside `plugin.toml`, if any.
    fn lib_file(dir: &Path) -> Option<PathBuf> {
        let ext = lib_extension();
        std::fs::read_dir(dir).ok()?.flatten().find_map(|e| {
            let p = e.path();
            (p.extension().and_then(|s| s.to_str()) == Some(ext)).then_some(p)
        })
    }

    /// Load a discovered package's `cdylib`, gating on the API version before
    /// any of its code runs. Approach B (see `docs/plugin-architecture.md`):
    /// the plugin hands back a boxed `BuiltinPlugin`; this assumes the package
    /// was built against the same toolchain and `ocs_plugin_api` version, which
    /// the version symbol enforces.
    ///
    /// # Safety
    /// Calls `dlopen`/`dlsym` on an arbitrary file and trusts its exported
    /// symbols' signatures. Only invoke on packages the user installed.
    pub fn load(p: &ExternalPlugin) -> Result<LoadedPlugin, String> {
        let path = lib_file(&p.dir).ok_or("no native library in package")?;
        unsafe {
            let lib = libloading::Library::new(&path).map_err(|e| e.to_string())?;

            let version: libloading::Symbol<extern "C" fn() -> u32> = lib
                .get(b"ocs_plugin_api_version")
                .map_err(|_| "missing ocs_plugin_api_version symbol".to_string())?;
            let v = version();
            if v != ocs_plugin_api::API_VERSION {
                return Err(format!(
                    "API version {v} != host {}",
                    ocs_plugin_api::API_VERSION
                ));
            }

            let register: libloading::Symbol<
                extern "C" fn() -> *mut Box<dyn BuiltinPlugin>,
            > = lib
                .get(b"ocs_plugin_register")
                .map_err(|_| "missing ocs_plugin_register symbol".to_string())?;
            let raw = register();
            if raw.is_null() {
                return Err("ocs_plugin_register returned null".into());
            }
            let plugin = *Box::from_raw(raw);
            let id = plugin.manifest().id.to_string();
            Ok(LoadedPlugin {
                plugin,
                _lib: lib,
                id,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_documented_keys() {
        let toml = r#"
[plugin]
id = "opencad.storm_sewer"
name = "Storm Sewer"
version = "0.2.0"
description = "Gravity storm-drain design"

[opencad]
api_version = 1
ribbon_order = 50
command_prefixes = ["SS_", "STORM_"]
"#;
        let p = parse_plugin_toml(toml).expect("parsed");
        assert_eq!(p.id, "opencad.storm_sewer");
        assert_eq!(p.name, "Storm Sewer");
        assert_eq!(p.version, "0.2.0");
        assert_eq!(p.api_version, 1);
        assert_eq!(p.ribbon_order, 50);
        assert_eq!(p.command_prefixes, vec!["SS_", "STORM_"]);
        assert!(p.api_compatible());
    }

    #[test]
    fn missing_id_is_rejected() {
        assert!(parse_plugin_toml("name = \"x\"").is_none());
    }

    #[test]
    fn incompatible_api_flagged() {
        let p = parse_plugin_toml("id=\"a\"\napi_version = 9999").unwrap();
        assert!(!p.api_compatible());
        assert!(!p.loadable());
    }
}
