use std::path::PathBuf;

#[cfg(target_os = "linux")]
use directories::UserDirs;

#[cfg(target_os = "linux")]
pub fn get_install_dir() -> Option<PathBuf> {
    if let Some(user_dirs) = UserDirs::new() {
        let mut install_path = PathBuf::from(user_dirs.home_dir());
        for path in [".steam", "debian-installation"] {
            install_path.push(path);
        }
        if install_path.is_dir() {
            Some(install_path)
        } else {
            None
        }
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
pub fn get_install_dir() -> Option<PathBuf> {
    let path = PathBuf::from("C:\\Program Files (x86)\\Steam");
    if path.is_dir() {
        Some(path)
    } else {
        None
    }
}

pub fn get_game_install_path(game_name: &str) -> Option<PathBuf> {
    if let Some(mut install_path) = get_install_dir() {
        for path in ["steamapps", "common", game_name] {
            install_path.push(path);
        }
        Some(install_path)
    } else {
        None
    }
}

#[cfg(target_os = "linux")]
pub fn get_wine_prefix_dir(game_id: i64) -> Option<PathBuf> {
    if let Some(mut install_path) = get_install_dir() {
        for path in ["steamapps", "compatdata", &game_id.to_string(), "pfx", "drive_c"] {
            install_path.push(path);
        }
        Some(install_path)
    } else {
        None
    }
}

#[cfg(target_os = "linux")]
pub fn get_wine_documents_dir(game_id: i64) -> Option<PathBuf> {
    if let Some(mut install_path) = get_wine_prefix_dir(game_id) {
        for path in ["users", "steamuser", "My Documents"] {
            install_path.push(path);
        }
        Some(install_path)
    } else {
        None
    }
}

#[cfg(target_os = "linux")]
pub fn get_wine_appdata_local_dir(game_id: i64) -> Option<PathBuf> {
    if let Some(mut install_path) = get_wine_prefix_dir(game_id) {
        for path in ["users", "steamuser", "AppData", "Local"] {
            install_path.push(path);
        }
        Some(install_path)
    } else {
        None
    }
}

