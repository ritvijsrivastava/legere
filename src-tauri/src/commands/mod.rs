pub mod articles;
pub mod import;
pub mod settings;
pub mod sources;
pub mod system;
pub mod update;
#[cfg(all(target_os = "android", feature = "apk-self-update"))]
pub mod update_android;
#[cfg(not(target_os = "android"))]
pub mod update_linux;
