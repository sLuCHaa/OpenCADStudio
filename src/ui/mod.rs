/// Single source of truth for UI row height (px).
/// Change this to scale the ribbon, layer manager rows, and property panel rows uniformly.
pub const ROW_H: f32 = 26.0;

pub mod about;
pub mod app_menu;
pub mod isolate_popup;
pub mod update_notice;
pub mod command_line;
pub mod dimstyle;
pub mod layers;
pub mod layout_manager;
pub mod mleaderstyle;
pub mod mlstyle;
pub mod open_progress;
pub mod overlay;
pub mod page_setup;
pub mod plotstyle;
pub mod properties;
pub mod ribbon;
pub mod scale_popup;
pub mod selection_filter_popup;
pub mod shortcuts;
pub mod snap_popup;
pub mod statusbar;
pub mod statusbar_config;
pub mod statusbar_menu;
pub mod tablestyle;
pub mod textstyle;
pub mod units_popup;

pub use app_menu::AppMenu;
pub use command_line::CommandLine;
pub use layers::LayerPanel;
pub use properties::PropertiesPanel;
pub use ribbon::Ribbon;
pub use statusbar::StatusBar;
