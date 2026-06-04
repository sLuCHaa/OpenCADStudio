//! Status-bar customization — which toggle pills are shown on the bar.
//!
//! The customization menu (opened from the bar's far-right handle) lists every
//! pill with a check mark next to the ones currently shown. Toggling a row
//! adds or removes that pill from the bar. The choice is persisted so it
//! survives across sessions.

use std::collections::HashSet;
use std::path::PathBuf;

/// Identifies a toggleable status-bar pill.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum StatusPill {
    Coords,
    Snap,
    Grid,
    Ortho,
    Lwt,
    Polar,
    Dyn,
    Otrack,
    Osnap,
    Space,
    Scale,
    Units,
    Transparency,
    Isolate,
    QuickProps,
    SelFilter,
    Vp,
    CleanScreen,
}

impl StatusPill {
    /// Every pill, in status-bar display order. Drives both the bar layout and
    /// the customization menu.
    pub const ALL: &'static [StatusPill] = &[
        StatusPill::Coords,
        StatusPill::Snap,
        StatusPill::Grid,
        StatusPill::Ortho,
        StatusPill::Lwt,
        StatusPill::Polar,
        StatusPill::Dyn,
        StatusPill::Otrack,
        StatusPill::Osnap,
        StatusPill::Space,
        StatusPill::Scale,
        StatusPill::Units,
        StatusPill::Transparency,
        StatusPill::Isolate,
        StatusPill::QuickProps,
        StatusPill::SelFilter,
        StatusPill::Vp,
        StatusPill::CleanScreen,
    ];

    /// Stable identifier used for persistence.
    pub fn id(self) -> &'static str {
        match self {
            StatusPill::Coords => "coords",
            StatusPill::Snap => "snap",
            StatusPill::Grid => "grid",
            StatusPill::Ortho => "ortho",
            StatusPill::Lwt => "lwt",
            StatusPill::Polar => "polar",
            StatusPill::Dyn => "dyn",
            StatusPill::Otrack => "otrack",
            StatusPill::Osnap => "osnap",
            StatusPill::Space => "space",
            StatusPill::Scale => "scale",
            StatusPill::Units => "units",
            StatusPill::Transparency => "transparency",
            StatusPill::Isolate => "isolate",
            StatusPill::QuickProps => "quickprops",
            StatusPill::SelFilter => "selfilter",
            StatusPill::Vp => "vp",
            StatusPill::CleanScreen => "cleanscreen",
        }
    }

    /// Label shown in the customization menu.
    pub fn label(self) -> &'static str {
        match self {
            StatusPill::Coords => "Coordinates",
            StatusPill::Snap => "Snap Mode",
            StatusPill::Grid => "Grid",
            StatusPill::Ortho => "Ortho Mode",
            StatusPill::Lwt => "Show Lineweight",
            StatusPill::Polar => "Polar Tracking",
            StatusPill::Dyn => "Dynamic Input",
            StatusPill::Otrack => "Object Snap Tracking",
            StatusPill::Osnap => "Object Snap",
            StatusPill::Space => "Model/Paper Space",
            StatusPill::Scale => "Annotation Scale",
            StatusPill::Units => "Drawing Units",
            StatusPill::Transparency => "Show Transparency",
            StatusPill::Isolate => "Isolate Objects",
            StatusPill::QuickProps => "Quick Properties",
            StatusPill::SelFilter => "Selection Filtering",
            StatusPill::Vp => "Viewport Count",
            StatusPill::CleanScreen => "Clean Screen",
        }
    }

    fn from_id(s: &str) -> Option<StatusPill> {
        StatusPill::ALL.iter().copied().find(|p| p.id() == s)
    }
}

/// Tracks which pills the user has hidden. Empty = every pill visible.
#[derive(Clone, Default)]
pub struct StatusBarConfig {
    hidden: HashSet<StatusPill>,
}

impl StatusBarConfig {
    /// Load the saved customization, or all-visible when none exists.
    pub fn load() -> Self {
        let mut hidden = HashSet::new();
        if let Some(path) = config_path() {
            if let Ok(body) = std::fs::read_to_string(path) {
                for line in body.lines() {
                    if let Some(p) = StatusPill::from_id(line.trim()) {
                        hidden.insert(p);
                    }
                }
            }
        }
        Self { hidden }
    }

    pub fn is_visible(&self, pill: StatusPill) -> bool {
        !self.hidden.contains(&pill)
    }

    /// Flip a pill's visibility and persist the change.
    pub fn toggle(&mut self, pill: StatusPill) {
        if !self.hidden.remove(&pill) {
            self.hidden.insert(pill);
        }
        self.save();
    }

    fn save(&self) {
        let Some(path) = config_path() else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let body: String = StatusPill::ALL
            .iter()
            .filter(|p| self.hidden.contains(p))
            .map(|p| p.id())
            .collect::<Vec<_>>()
            .join("\n");
        let _ = std::fs::write(path, body);
    }
}

/// `<config-dir>/OpenCADStudio/statusbar.txt`, matching the recent-files store.
fn config_path() -> Option<PathBuf> {
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
    Some(p.join("statusbar.txt"))
}
