use std::{fs, io};

use log::LevelFilter;
use tauri::AppHandle;

mod commands;
mod git_ops;
mod logger;
mod profiles;

pub use logger::GuiLogger;

pub const EVENT_OUTPUT: &str = "commit-output";
pub const EVENT_DONE: &str = "commit-done";

const DEFAULT_DEBUG: bool = false;

static GUI_LOGGER: GuiLogger = GuiLogger::new();

#[tauri::command]
fn set_debug_logging(app: AppHandle, debug: bool) {
    GUI_LOGGER.configure(app, debug);
}

pub fn run() {
    log::set_logger(&GUI_LOGGER).expect("failed to initialize GUI logger");
    log::set_max_level(LevelFilter::Info);
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            GUI_LOGGER.configure(app.handle().clone(), DEFAULT_DEBUG);
            if let Ok(settings_path) =
                profiles::app_data_file(app.handle(), "settings.json")
            {
                match fs::remove_file(settings_path) {
                    Ok(()) => {}
                    Err(err) if err.kind() == io::ErrorKind::NotFound => {}
                    Err(err) => log::warn!("Failed to remove legacy settings.json: {}", err),
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::pick_directory,
            profiles::get_profiles,
            profiles::upsert_profile,
            profiles::delete_profile,
            set_debug_logging,
            commands::run_commit,
            commands::run_checkout,
            git_ops::check_repo_exists,
            commands::run_clone,
            commands::run_pull,
            commands::run_push,
            git_ops::get_commits,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::profiles::{Profile, normalize_profiles};

    #[test]
    fn normalize_profiles_filters_empty_save_dirs_and_keeps_first_duplicate() {
        let normalized = normalize_profiles(vec![
            Profile { save_dir: "/b".into(), mc_version: "1.20.1".into(), branch: "main".into(), remote_url: String::new() },
            Profile { save_dir: "".into(), mc_version: "1.21.1".into(), branch: "empty".into(), remote_url: String::new() },
            Profile { save_dir: "   ".into(), mc_version: "1.21.2".into(), branch: "blank".into(), remote_url: String::new() },
            Profile { save_dir: "/a".into(), mc_version: "1.19.4".into(), branch: "stable".into(), remote_url: String::new() },
            Profile { save_dir: "/b".into(), mc_version: "1.21.4".into(), branch: "newer".into(), remote_url: String::new() },
        ]);

        assert_eq!(normalized.len(), 2);
        assert_eq!(normalized[0].save_dir, "/a");
        assert_eq!(normalized[0].branch, "stable");
        assert_eq!(normalized[1].save_dir, "/b");
        assert_eq!(normalized[1].branch, "main");
    }
}
