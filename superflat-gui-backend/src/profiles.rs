use std::{
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, path::BaseDirectory};

pub const PROFILES_FILE: &str = "profiles.json";

#[derive(Serialize, Deserialize, Clone)]
pub struct Profile {
    pub save_dir: String,
    pub mc_version: String,
    pub branch: String,
    #[serde(default)]
    pub remote_url: String,
    pub updated_at: String,
}

fn now_iso() -> String {
    // Use JS-compatible ISO 8601 format via std time
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (y, mo, d, h, mi, s) = epoch_to_ymd_hms(secs);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, mo, d, h, mi, s)
}

fn epoch_to_ymd_hms(secs: u64) -> (u64, u64, u64, u64, u64, u64) {
    let s = secs % 60;
    let mi = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let days = secs / 86400;
    // Compute year/month/day from days since epoch
    let mut year = 1970u64;
    let mut remaining = days;
    loop {
        let leap = is_leap(year);
        let days_in_year = if leap { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        year += 1;
    }
    let leap = is_leap(year);
    let months = [31u64, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u64;
    for &dim in &months {
        if remaining < dim {
            break;
        }
        remaining -= dim;
        month += 1;
    }
    (year, month, remaining + 1, h, mi, s)
}

fn is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

pub fn normalize_profiles(profiles: Vec<Profile>) -> Vec<Profile> {
    let mut seen_save_dirs = HashSet::with_capacity(profiles.len());
    let mut normalized = Vec::with_capacity(profiles.len());

    for profile in profiles {
        if profile.save_dir.trim().is_empty() {
            continue;
        }
        if seen_save_dirs.insert(profile.save_dir.clone()) {
            normalized.push(profile);
        }
    }

    normalized.sort_by(|a, b| a.save_dir.cmp(&b.save_dir));
    normalized
}

pub fn app_data_file(app: &AppHandle, file_name: &str) -> io::Result<PathBuf> {
    app.path()
        .resolve(file_name, BaseDirectory::AppData)
        .map_err(io::Error::other)
}

pub fn read_profiles_file(path: &Path) -> io::Result<Vec<Profile>> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice::<Vec<Profile>>(&bytes)
            .map(normalize_profiles)
            .map_err(io::Error::other),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(err) => Err(err),
    }
}

pub fn write_profiles_file(path: &Path, profiles: &[Profile]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let normalized = normalize_profiles(profiles.to_vec());
    let bytes = serde_json::to_vec_pretty(&normalized).map_err(io::Error::other)?;
    fs::write(path, bytes)
}

#[tauri::command]
pub fn get_profiles(app: AppHandle) -> Result<Vec<Profile>, String> {
    let path = app_data_file(&app, PROFILES_FILE).map_err(|e| e.to_string())?;
    read_profiles_file(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn upsert_profile(app: AppHandle, mut profile: Profile) -> Result<(), String> {
    profile.updated_at = now_iso();
    let path = app_data_file(&app, PROFILES_FILE).map_err(|e| e.to_string())?;
    let mut profiles = read_profiles_file(&path).map_err(|e| e.to_string())?;

    if let Some(existing) = profiles.iter_mut().find(|p| p.save_dir == profile.save_dir) {
        *existing = profile;
    } else {
        profiles.push(profile);
    }

    write_profiles_file(&path, &profiles).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_profile(app: AppHandle, save_dir: String) -> Result<(), String> {
    let path = app_data_file(&app, PROFILES_FILE).map_err(|e| e.to_string())?;
    let profiles = read_profiles_file(&path).map_err(|e| e.to_string())?;
    let profiles: Vec<Profile> = profiles
        .into_iter()
        .filter(|p| p.save_dir != save_dir)
        .collect();
    write_profiles_file(&path, &profiles).map_err(|e| e.to_string())
}
