# ocs_example_plugin

Reference **dynamically-loaded** add-on for Open CAD Studio. It depends only on
`ocs_plugin_api` (with the `host` feature) — never on the `OpenCADStudio`
binary — so it shows the full surface an out-of-tree plugin targets: a
`PluginManifest`, a `CadModule` ribbon tab, a `BuiltinPlugin` entry point, and
the `export_plugin!` C-ABI export.

## Build & install

```sh
cargo build -p ocs_example_plugin            # → target/debug/libocs_example_plugin.so
```

Copy the library and `plugin.toml` into a folder named after the plugin id under
the user plugins directory:

```
<config>/OpenCADStudio/plugins/opencad.example/
  plugin.toml
  libocs_example_plugin.so      # .dll on Windows, .dylib on macOS
```

`<config>` is `%APPDATA%` (Windows), `~/Library/Application Support` (macOS), or
`$XDG_CONFIG_HOME` / `~/.config` (Linux).

Restart Open CAD Studio. The host loads the cdylib at startup (after checking
`ocs_plugin_api_version`), adds the **Example** ribbon tab, and routes `EX_`
commands to it. `PLUGINS` lists it under *External* as **Loaded**; run `EX_HELLO`
to see it respond.

## Contract

- `ocs_plugin_api::export_plugin!(MyPlugin)` emits `ocs_plugin_api_version()` and
  `ocs_plugin_register()`.
- The package must be built with the **same toolchain and `ocs_plugin_api`
  version** as the host (approach B — the version symbol enforces the latter).
