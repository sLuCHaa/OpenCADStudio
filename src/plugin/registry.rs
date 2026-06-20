// Plugin registry — external (dynamically-loaded) plugins only. OpenCADStudio
// ships no built-in add-ons; every plugin is a cdylib loaded from the plugins
// folder at startup (see `external`) and the marketplace installs them there.

use crate::app::OpenCADStudio;
use crate::modules::{registry as core_registry, CadModule};

/// Core ribbon tabs plus every loaded external add-on tab.
pub fn all_ribbon_modules() -> Vec<Box<dyn CadModule>> {
    ribbon_modules_enabled(&rustc_hash::FxHashSet::default())
}

/// Core ribbon tabs plus the tabs of loaded external plugins whose id is **not**
/// in `disabled` (sorted by `manifest.ribbon_order`).
pub fn ribbon_modules_enabled(
    disabled: &rustc_hash::FxHashSet<String>,
) -> Vec<Box<dyn CadModule>> {
    let mut core = core_registry::all_modules();
    // Dynamically-loaded external plugins contribute tabs (their libraries stay
    // resident for the session, so these vtables remain valid).
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut addons: Vec<(i32, Box<dyn CadModule>)> = Vec::new();
        crate::plugin::external::with_loaded(|loaded| {
            for lp in loaded {
                if disabled.contains(lp.id.as_str()) {
                    continue;
                }
                // Guard the plugin's ribbon build so a panic there can't take
                // down the host — the plugin just contributes no tab. (#145)
                if let Some(entry) = crate::plugin::guard("ribbon", || {
                    (lp.plugin().manifest().ribbon_order, lp.plugin().ribbon())
                }) {
                    addons.push(entry);
                }
            }
        });
        addons.sort_by_key(|(order, _)| *order);
        core.extend(addons.into_iter().map(|(_, ribbon)| ribbon));
    }
    let _ = disabled;
    core
}

/// Dispatch `cmd` to a loaded external plugin (skipping disabled ones).
/// Returns true if one handled it.
pub(crate) fn try_dispatch(app: &mut OpenCADStudio, tab: usize, cmd: &str) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use super::host::HostSession;
        let disabled = app.disabled_plugin_ids();
        let handled = crate::plugin::external::with_loaded(|loaded| {
            let mut host = HostSession::new(app, tab);
            for lp in loaded {
                if disabled.contains(lp.id.as_str()) {
                    continue;
                }
                // A panic inside the plugin's dispatch must not crash the host;
                // treat a panicking plugin as "didn't handle it". (#145)
                if crate::plugin::guard("dispatch", || lp.plugin().dispatch(&mut host, cmd))
                    .unwrap_or(false)
                {
                    return true;
                }
            }
            false
        });
        if handled {
            return true;
        }
    }
    let _ = (app, tab, cmd);
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ribbon_is_core_only_without_external_plugins() {
        // No external plugins are loaded under test, so the ribbon is exactly
        // the built-in core tabs.
        let titles: Vec<&str> = all_ribbon_modules().iter().map(|m| m.title()).collect();
        assert!(!titles.is_empty(), "expected core ribbon tabs");
        assert_eq!(titles.len(), core_registry::all_modules().len());
    }
}
