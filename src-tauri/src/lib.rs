pub mod bridge;
pub mod i18n;
pub mod media;
pub mod navigation;
pub mod notify;
pub mod settings;
pub mod shell;
pub mod tray;
pub mod user_agent;

#[cfg(feature = "e2e")]
mod e2e;

pub use shell::run;
