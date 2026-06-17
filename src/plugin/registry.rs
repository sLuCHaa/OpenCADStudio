// Compile-time plugin registry via `inventory`.

use super::host::{BuiltinPlugin, HostSession};
use super::manifest::PluginManifest;
use crate::app::OpenCADStudio;
use crate::modules::{registry as core_registry, CadModule};

pub struct PluginRegistration {
    pub construct: fn() -> Box<dyn BuiltinPlugin>,
}

inventory::collect!(PluginRegistration);

/// Construct every registered built-in plugin (once per process).
pub fn all_plugins() -> Vec<Box<dyn BuiltinPlugin>> {
    inventory::iter::<PluginRegistration>
        .into_iter()
        .map(|r| (r.construct)())
        .collect()
}

/// Static manifest of every installed add-on, sorted by `ribbon_order` then id
/// for a stable display. Used by the plugin manager window.
pub fn installed_manifests() -> Vec<&'static PluginManifest> {
    let mut manifests: Vec<&'static PluginManifest> =
        all_plugins().iter().map(|p| p.manifest()).collect();
    manifests.sort_by(|a, b| a.ribbon_order.cmp(&b.ribbon_order).then(a.id.cmp(b.id)));
    manifests
}

/// Core ribbon tabs plus *every* add-on tab (sorted by `manifest.ribbon_order`).
pub fn all_ribbon_modules() -> Vec<Box<dyn CadModule>> {
    ribbon_modules_enabled(&rustc_hash::FxHashSet::default())
}

/// Core ribbon tabs plus add-on tabs whose plugin id is **not** in `disabled`
/// (sorted by `manifest.ribbon_order`). Used by the Plugin Manager toggle.
pub fn ribbon_modules_enabled(
    disabled: &rustc_hash::FxHashSet<String>,
) -> Vec<Box<dyn CadModule>> {
    let mut core = core_registry::all_modules();
    let mut addons: Vec<(i32, Box<dyn CadModule>)> = all_plugins()
        .into_iter()
        .filter(|p| !disabled.contains(p.manifest().id))
        .map(|p| (p.manifest().ribbon_order, p.ribbon()))
        .collect();
    addons.sort_by_key(|(order, _)| *order);
    core.extend(addons.into_iter().map(|(_, ribbon)| ribbon));
    core
}

/// Try each *enabled* plugin until one handles `cmd`. Returns true if handled.
/// Disabled plugins (toggled off in the Plugin Manager) are skipped.
pub(crate) fn try_dispatch(app: &mut OpenCADStudio, tab: usize, cmd: &str) -> bool {
    let disabled = app.disabled_plugin_ids();
    let mut host = HostSession::new(app, tab);
    for plugin in all_plugins() {
        if disabled.contains(plugin.manifest().id) {
            continue;
        }
        if plugin.dispatch(&mut host, cmd) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::OpenCADStudio;
    #[test]
    fn discovers_registered_plugins() {
        let plugins = all_plugins();
        assert!(
            !plugins.is_empty(),
            "expected at least one PluginRegistration (demo_plugin)"
        );
        assert!(
            plugins
                .iter()
                .any(|p| p.manifest().id == "opencad.demo_plugin"),
            "demo_plugin missing; ids: {:?}",
            plugins.iter().map(|p| p.manifest().id).collect::<Vec<_>>()
        );
    }

    #[test]
    fn installed_manifests_lists_demo_plugin() {
        let manifests = installed_manifests();
        assert!(
            manifests.iter().any(|m| m.id == "opencad.demo_plugin"),
            "ids: {:?}",
            manifests.iter().map(|m| m.id).collect::<Vec<_>>()
        );
        // Sorted by (ribbon_order, id) — verify non-decreasing order.
        let mut prev: Option<(i32, &str)> = None;
        for m in &manifests {
            let key = (m.ribbon_order, m.id);
            if let Some(p) = prev {
                assert!(p <= key, "manifests not sorted: {p:?} then {key:?}");
            }
            prev = Some(key);
        }
    }

    #[test]
    fn addon_ribbon_tabs_merge_after_core() {
        let titles: Vec<&str> = all_ribbon_modules().iter().map(|m| m.title()).collect();
        assert!(titles.contains(&"Demo Plugin"), "ribbon tabs: {titles:?}");
        let core = core_registry::all_modules();
        assert_eq!(titles.len(), core.len() + all_plugins().len());
    }

    #[test]
    fn disabled_plugin_drops_its_ribbon_tab() {
        let mut disabled = rustc_hash::FxHashSet::default();
        disabled.insert("opencad.demo_plugin".to_string());
        let titles: Vec<&str> = ribbon_modules_enabled(&disabled)
            .iter()
            .map(|m| m.title())
            .collect();
        assert!(
            !titles.contains(&"Demo Plugin"),
            "disabled plugin still present: {titles:?}"
        );
        // Only the add-on tab is dropped; core tabs stay.
        assert_eq!(titles.len(), core_registry::all_modules().len());
    }

    #[test]
    fn try_dispatch_routes_demo_command() {
        let mut app = OpenCADStudio::new_for_test();
        assert!(try_dispatch(&mut app, 0, "DP_HELLO"));
        let info = app.command_history_info();
        assert!(
            info.iter().any(|t| t.contains("demo_plugin") && t.contains("plugin host OK")),
            "info history: {info:?}"
        );
    }

    #[test]
    fn unknown_plugin_command_falls_through() {
        let mut app = OpenCADStudio::new_for_test();
        assert!(!try_dispatch(&mut app, 0, "DP_NOPE"));
    }
}