pub const APP_ID: &str = "io.github.weversonl.GnomeQS";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GETTEXT_PACKAGE: &str = "gnomeqs";

/// Directory containing compiled GSettings schema (set by build.rs).
#[cfg(debug_assertions)]
pub const SCHEMA_DIR: &str = env!("SCHEMA_DIR");

/// Directory containing compiled .mo locale files (set by build.rs).
#[cfg(debug_assertions)]
pub const LOCALE_DIR: &str = env!("LOCALE_DIR");
#[cfg(not(debug_assertions))]
pub const LOCALE_DIR: &str = "/usr/share/locale";
