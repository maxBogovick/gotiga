use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::AppHandle;
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub server_url: String,
    pub api_key: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:3000".to_string(),
            api_key: "".to_string(),
        }
    }
}

pub struct SettingsService {
    config_path: PathBuf,
    settings: Mutex<AppSettings>,
}

impl SettingsService {
    pub fn new(app: &AppHandle) -> Self {
        let app_data_dir = app
            .path()
            .app_data_dir()
            .expect("Failed to get app data dir");
        let config_path = app_data_dir.join("settings.json");

        let settings = if config_path.exists() {
            let content = fs::read_to_string(&config_path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            AppSettings::default()
        };

        Self {
            config_path,
            settings: Mutex::new(settings),
        }
    }

    pub fn get_settings(&self) -> AppSettings {
        self.settings.lock().unwrap().clone()
    }

    pub fn save_settings(&self, new_settings: AppSettings) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&new_settings).map_err(|e| e.to_string())?;
        fs::write(&self.config_path, json).map_err(|e| e.to_string())?;

        let mut lock = self.settings.lock().unwrap();
        *lock = new_settings;
        Ok(())
    }
}
