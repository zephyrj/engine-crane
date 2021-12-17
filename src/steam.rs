use std::path::PathBuf;

#[cfg(target_os = "linux")]
use directories::UserDirs;

#[cfg(target_os = "linux")]
pub fn get_install_path(&str: game_name) -> Option<PathBuf> {
    if let Some(user_dirs) = UserDirs::new() {
        let mut install_path = PathBuf::from(user_dirs.home_dir());
        for path in [".steam", "debian-installation", "steamapps", "common", game_name] {
            install_path.push(path);
        }
        Some(install_path)
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
pub fn get_install_path(game_name: &str) -> Option<PathBuf> {
    let path = PathBuf::from(format!("C:\\Program Files (x86)\\Steam\\steamapps\\common\\{}",
                                     game_name));
    if path.is_dir() {
        Some(path)
    } else {
        None
    }
}
