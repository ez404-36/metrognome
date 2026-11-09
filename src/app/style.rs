pub const CSS: &str = include_str!("style.css");

/// Compiled from `icons.gresource.xml` by `build.rs` when the UI is enabled.
pub const ICONS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icons.gresource"));

/// Registered with GTK's display-local icon theme at startup.
pub const ICON_RESOURCE_PATH: &str = "/io/github/takto/icons/hicolor";
