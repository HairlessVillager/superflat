use std::{fs, io, sync::LazyLock};

use log::LevelFilter;
use tauri::AppHandle;

mod commands;
mod git_ops;
mod logger;
mod profiles;

pub use logger::GuiLogger;

pub const EVENT_OUTPUT: &str = "sf-log";
pub const EVENT_DONE: &str = "sf-done";

static GUI_LOGGER: LazyLock<GuiLogger> = LazyLock::new(GuiLogger::new);

#[tauri::command]
fn get_log_path(app: AppHandle) -> Result<String, String> {
    profiles::app_data_dir(&app)
        .map(|p| p.join(logger::LOG_DIR))
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn log_file_exists(app: AppHandle) -> bool {
    let _ = app;
    GUI_LOGGER.get_current_log_path()
        .map(|p| p.exists())
        .unwrap_or(false)
}

#[tauri::command]
fn open_log_file(_app: AppHandle) -> Result<(), String> {
    use log::Log;
    GUI_LOGGER.flush();
    let path = GUI_LOGGER.get_current_log_path().ok_or("No log file available")?;
    log::debug!("Opening log file in file manager: {:?}", path);

    #[cfg(target_os = "linux")]
    {
        // On Linux, try file managers that support selecting files
        // nautilus (GNOME), dolphin (KDE), thunar (XFCE), pcmanfm (LXDE)
        let parent = path.parent().ok_or("Failed to get parent directory")?;

        // Try nautilus first (GNOME Files)
        if let Ok(child) = std::process::Command::new("nautilus")
            .arg("--select")
            .arg(&path)
            .spawn()
        {
            drop(child);
            return Ok(());
        }
        // Try dolphin (KDE)
        if let Ok(child) = std::process::Command::new("dolphin")
            .arg("--select")
            .arg(&path)
            .spawn()
        {
            drop(child);
            return Ok(());
        }
        // Try thunar (XFCE)
        if let Ok(child) = std::process::Command::new("thunar")
            .arg("--select")
            .arg(&path)
            .spawn()
        {
            drop(child);
            return Ok(());
        }
        // Try pcmanfm (LXDE/LXQt)
        if let Ok(child) = std::process::Command::new("pcmanfm")
            .arg("--select-file")
            .arg(&path)
            .spawn()
        {
            drop(child);
            return Ok(());
        }
        // Fallback: open the directory without selecting
        if let Ok(child) = std::process::Command::new("gio")
            .arg("open")
            .arg(parent)
            .spawn()
        {
            drop(child);
            return Ok(());
        }
        std::process::Command::new("xdg-open")
            .arg(parent)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        // On macOS, use open -R to reveal the file in Finder
        std::process::Command::new("open")
            .arg("-R")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, use explorer /select to open and select the file
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }


    Ok(())
}

#[tauri::command]
fn window_minimize(window: tauri::Window) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())
}

#[tauri::command]
fn window_toggle_maximize(window: tauri::Window) -> Result<(), String> {
    if window.is_maximized().map_err(|e| e.to_string())? {
        window.unmaximize().map_err(|e| e.to_string())
    } else {
        window.maximize().map_err(|e| e.to_string())
    }
}

#[tauri::command]
fn window_close(window: tauri::Window) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

#[tauri::command]
fn window_start_dragging(window: tauri::Window) -> Result<(), String> {
    window.start_dragging().map_err(|e| e.to_string())
}

pub fn flush_log() {
    use log::Log;
    GUI_LOGGER.flush();
}

pub fn reset_op_start() {
    GUI_LOGGER.reset_op_start();
}

pub fn run() {
    log::set_logger(&*GUI_LOGGER).expect("failed to initialize GUI logger");
    log::set_max_level(LevelFilter::Debug);
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = profiles::app_data_dir(app.handle())
                .expect("failed to resolve app data dir");
            GUI_LOGGER.configure(app.handle().clone(), app_data_dir);
            if let Ok(settings_path) = profiles::app_data_file(app.handle(), "settings.json") {
                match fs::remove_file(settings_path) {
                    Ok(()) => {}
                    Err(err) if err.kind() == io::ErrorKind::NotFound => {}
                    Err(err) => log::warn!("Failed to remove legacy settings.json: {}", err),
                }
            }
            Ok(())
        })
        .on_window_event(|_window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                log::logger().flush();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::pick_directory,
            profiles::get_profiles,
            profiles::upsert_profile,
            profiles::delete_profile,
            get_log_path,
            log_file_exists,
            open_log_file,
            window_minimize,
            window_toggle_maximize,
            window_close,
            window_start_dragging,
            commands::run_commit,
            commands::run_checkout,
            commands::check_bak_exists,
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
            Profile {
                save_dir: "/b".into(),
                mc_version: "1.20.1".into(),
                branch: "main".into(),
                remote_url: String::new(),
                updated_at: String::new(),
            },
            Profile {
                save_dir: "".into(),
                mc_version: "1.21.1".into(),
                branch: "empty".into(),
                remote_url: String::new(),
                updated_at: String::new(),
            },
            Profile {
                save_dir: "   ".into(),
                mc_version: "1.21.2".into(),
                branch: "blank".into(),
                remote_url: String::new(),
                updated_at: String::new(),
            },
            Profile {
                save_dir: "/a".into(),
                mc_version: "1.19.4".into(),
                branch: "stable".into(),
                remote_url: String::new(),
                updated_at: String::new(),
            },
            Profile {
                save_dir: "/b".into(),
                mc_version: "1.21.4".into(),
                branch: "newer".into(),
                remote_url: String::new(),
                updated_at: String::new(),
            },
        ]);

        assert_eq!(normalized.len(), 2);
        assert_eq!(normalized[0].save_dir, "/a");
        assert_eq!(normalized[0].branch, "stable");
        assert_eq!(normalized[1].save_dir, "/b");
        assert_eq!(normalized[1].branch, "main");
    }
}
