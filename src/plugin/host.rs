// Plugin traits — HostSession lives in `app::plugin_host` (same-crate field
// access) and implements the stable `HostApi` contract plugins target.

pub(crate) use crate::app::plugin_host::HostSession;
/// The stable contract types a plugin targets. `BuiltinPlugin` (the package
/// entry point) and `HostApi` (the runtime surface its `dispatch` receives)
/// both live in `ocs_plugin_api` so in-tree and out-of-tree add-ons implement
/// the same trait. See `docs/plugin-architecture.md`.
pub use ocs_plugin_api::host::{BuiltinPlugin, HostApi};