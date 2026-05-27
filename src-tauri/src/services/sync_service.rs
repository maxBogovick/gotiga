use crate::db::repository::Repository;
use crate::db::Database;
use crate::models::FigurineDto;
use crate::services::file_service::FileService;
use crate::services::settings_service::AppSettings;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use tauri::AppHandle;

#[derive(Deserialize)]
struct UploadResponse {
    #[serde(rename = "relativePath")]
    relative_path: String,
}

pub struct SyncService {
    file_service: FileService,
}

impl SyncService {
    pub fn new(app: &AppHandle) -> Self {
        Self {
            file_service: FileService::new(app),
        }
    }

    // ADMIN: Push a single figurine to the server
    pub async fn push_figurine(
        &self,
        mut dto: FigurineDto,
        settings: &AppSettings,
    ) -> Result<String, String> {
        if settings.server_url.is_empty() {
            return Err("Server URL not configured".to_string());
        }

        let client = reqwest::Client::new();
        let api_key = &settings.api_key;
        let base_url = &settings.server_url;

        // Helper to upload file if it is local (starts with cabinet:// or is a local path)
        // Returns the remote relative path or original if http
        let upload_helper = |path: Option<String>| async {
            match path {
                Some(p) => {
                    if p.starts_with("http") {
                        Ok::<Option<String>, String>(Some(p))
                    } else {
                        // It's local. Extract real path.
                        // p might be "cabinet://localhost/images/..." or "images/..."
                        let clean = if p.starts_with("cabinet://") {
                            p.replace("cabinet://localhost/", "")
                        } else {
                            p
                        };

                        let full_path = self.file_service.get_full_path(&clean);
                        if full_path.exists() {
                            let remote_path = self
                                .upload_file(&client, base_url, api_key, &full_path)
                                .await?;
                            Ok::<Option<String>, String>(Some(remote_path))
                        } else {
                            // File missing locally? Keep as is or null?
                            // Keep as is, maybe server has it
                            Ok::<Option<String>, String>(Some(clean))
                        }
                    }
                }
                None => Ok::<Option<String>, String>(None),
            }
        };

        // 1. Upload Assets
        dto.video_url = upload_helper(dto.video_url).await?;
        dto.ambience_path = upload_helper(dto.ambience_path).await?;

        let mut new_images = Vec::new();
        for mut img in dto.images {
            img.url = upload_helper(Some(img.url)).await?.unwrap_or_default();
            new_images.push(img);
        }
        dto.images = new_images;

        let mut new_steps = Vec::new();
        for mut step in dto.process_steps {
            step.image_url = upload_helper(Some(step.image_url))
                .await?
                .unwrap_or_default();
            new_steps.push(step);
        }
        dto.process_steps = new_steps;

        // 2. Upsert DTO
        let url = format!("{}/api/v1/figurines", base_url);
        let res = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&dto)
            .send()
            .await
            .map_err(|e| format!("Failed to send figurine: {}", e))?;

        if !res.status().is_success() {
            let txt = res.text().await.unwrap_or_default();
            return Err(format!("Server error: {}", txt));
        }

        Ok("Successfully pushed to server".to_string())
    }

    async fn upload_file(
        &self,
        client: &reqwest::Client,
        base_url: &str,
        api_key: &str,
        file_path: &Path,
    ) -> Result<String, String> {
        let file_name = file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let file_content = fs::read(file_path).map_err(|e| e.to_string())?;

        let part = reqwest::multipart::Part::bytes(file_content).file_name(file_name);

        let form = reqwest::multipart::Form::new().part("file", part);

        let url = format!("{}/api/v1/upload", base_url);
        let res = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| format!("Upload failed: {}", e))?;

        if !res.status().is_success() {
            let txt = res.text().await.unwrap_or_default();
            return Err(format!("Upload error: {}", txt));
        }

        let json: UploadResponse = res.json().await.map_err(|e| e.to_string())?;
        Ok(json.relative_path)
    }

    // ADMIN: Push Full Release (Portable BLOB Database)
    pub async fn push_full_release(
        &self,
        _db: &Database,
        settings: &AppSettings,
    ) -> Result<String, String> {
        if settings.server_url.is_empty() {
            return Err("Server URL not configured".to_string());
        }

        // 1. Создаем временную копию базы для упаковки
        let app_data = self.file_service.get_base_path_string();
        let main_db_path = Path::new(&app_data).join("cabinet.db");
        let export_db_path = Path::new(&app_data).join("export_temp.db");

        if export_db_path.exists() {
            fs::remove_file(&export_db_path).map_err(|e| e.to_string())?;
        }
        fs::copy(&main_db_path, &export_db_path).map_err(|e| e.to_string())?;

        // 2. Упаковываем файлы в BLOB
        {
            let conn = rusqlite::Connection::open(&export_db_path).map_err(|e| e.to_string())?;
            let repo = Repository::new(&conn);

            // А. Фигурки (Аудио/Видео)
            let figurines = repo.get_all_figurines().map_err(|e| e.to_string())?;
            for f in figurines {
                let ambience = f
                    .ambience_path
                    .as_ref()
                    .and_then(|p| fs::read(self.file_service.get_full_path(p)).ok());
                let video = f
                    .video_url
                    .as_ref()
                    .and_then(|p| fs::read(self.file_service.get_full_path(p)).ok());
                if ambience.is_some() || video.is_some() {
                    repo.update_figurine_blobs(&f.id, ambience, video)
                        .map_err(|e| e.to_string())?;
                }
            }

            // Б. Изображения
            let images = repo.get_all_images().map_err(|e| e.to_string())?;
            for img in images {
                if let Ok(data) = fs::read(self.file_service.get_full_path(&img.file_path)) {
                    let original_data = img
                        .original_path
                        .as_ref()
                        .and_then(|p| fs::read(self.file_service.get_full_path(p)).ok());
                    let thumb_data = img
                        .thumb_path
                        .as_ref()
                        .and_then(|p| fs::read(self.file_service.get_full_path(p)).ok());
                    repo.update_image_blob(&img.id, data, original_data, thumb_data)
                        .map_err(|e| e.to_string())?;
                }
            }

            // В. Этапы (Гримуар)
            // Нам нужно вытащить все этапы для всех фигурок
            let mut all_steps = Vec::new();
            for f in &repo.get_all_figurines().map_err(|e| e.to_string())? {
                all_steps.extend(
                    repo.get_process_steps_for_figurine(&f.id)
                        .map_err(|e| e.to_string())?,
                );
            }
            for step in all_steps {
                if let Ok(data) = fs::read(self.file_service.get_full_path(&step.image_path)) {
                    repo.update_step_blob(&step.id, data)
                        .map_err(|e| e.to_string())?;
                }
            }

            // Г. Мастерская
            let workshop = repo
                .get_texts_by_category("workshop")
                .map_err(|e| e.to_string())?;
            for item in workshop {
                if let Some(path) = item.image_path {
                    if let Ok(data) = fs::read(self.file_service.get_full_path(&path)) {
                        repo.update_text_blob(&item.id, data)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }

            // Д. Общие ресурсы приложения (например, главный фон)
            for (key, path) in repo.get_app_resources().map_err(|e| e.to_string())? {
                if is_external_media_path(&path) {
                    continue;
                }
                if let Ok(data) = fs::read(self.file_service.get_full_path(&path)) {
                    repo.update_app_resource_blob(&key, data)
                        .map_err(|e| e.to_string())?;
                }
            }

            let missing = self
                .verify_release_media(&repo)
                .map_err(|e| e.to_string())?;
            if !missing.is_empty() {
                return Err(format!(
                    "Release media verification failed. Missing {} item(s): {}",
                    missing.len(),
                    missing.join(", ")
                ));
            }
        }

        // 3. Отправляем файл на сервер
        let client = reqwest::Client::new();
        let file_content = fs::read(&export_db_path).map_err(|e| e.to_string())?;
        let part = reqwest::multipart::Part::bytes(file_content).file_name("latest.db");
        let form = reqwest::multipart::Form::new().part("file", part);

        let url = format!("{}/api/v1/release/db", settings.server_url);
        let res = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", settings.api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| format!("Upload failed: {}", e))?;

        if !res.status().is_success() {
            let txt = res.text().await.unwrap_or_default();
            return Err(format!("Server error: {}", txt));
        }

        // Чистим за собой
        fs::remove_file(&export_db_path).ok();

        Ok("Database release exported successfully".to_string())
    }

    fn verify_release_media(&self, repo: &Repository<'_>) -> Result<Vec<String>, rusqlite::Error> {
        let mut missing = Vec::new();

        for f in repo.get_all_figurines()? {
            if let Some(path) = f.ambience_path.as_ref() {
                if !is_external_media_path(path)
                    && !self.file_service.file_exists(path)
                    && !repo.figurine_has_ambience_blob(&f.id)?
                {
                    missing.push(format!("figurine:{} ambience {}", f.id, path));
                }
            }

            if let Some(path) = f.video_url.as_ref() {
                if !is_external_media_path(path)
                    && !self.file_service.file_exists(path)
                    && !repo.figurine_has_video_blob(&f.id)?
                {
                    missing.push(format!("figurine:{} video {}", f.id, path));
                }
            }
        }

        for image in repo.get_all_images()? {
            if !is_external_media_path(&image.file_path)
                && !self.file_service.file_exists(&image.file_path)
                && !repo.image_has_blob(&image.id)?
            {
                missing.push(format!("image:{} {}", image.id, image.file_path));
            }

            if let Some(path) = image.original_path.as_ref() {
                if !is_external_media_path(path)
                    && !self.file_service.file_exists(path)
                    && !repo.image_has_original_blob(&image.id)?
                {
                    missing.push(format!("image:{} original {}", image.id, path));
                }
            }

            if let Some(path) = image.thumb_path.as_ref() {
                if !is_external_media_path(path)
                    && !self.file_service.file_exists(path)
                    && !repo.image_has_thumb_blob(&image.id)?
                {
                    missing.push(format!("image:{} thumb {}", image.id, path));
                }
            }
        }

        for f in repo.get_all_figurines()? {
            for step in repo.get_process_steps_for_figurine(&f.id)? {
                if !is_external_media_path(&step.image_path)
                    && !self.file_service.file_exists(&step.image_path)
                    && !repo.step_has_blob(&step.id)?
                {
                    missing.push(format!("step:{} {}", step.id, step.image_path));
                }
            }
        }

        for item in repo.get_texts_by_category("workshop")? {
            if let Some(path) = item.image_path.as_ref() {
                if !is_external_media_path(path)
                    && !self.file_service.file_exists(path)
                    && !repo.text_has_blob(&item.id)?
                {
                    missing.push(format!("workshop:{} {}", item.id, path));
                }
            }
        }

        for (key, path) in repo.get_app_resources()? {
            if key == "author_profile" || is_external_media_path(&path) {
                continue;
            }
            if !self.file_service.file_exists(&path) && !repo.app_resource_has_blob(&key)? {
                missing.push(format!("resource:{} {}", key, path));
            }
        }

        Ok(missing)
    }

    /// CLIENT: Pull Full Database from server
    pub async fn pull_updates(
        &self,
        _db: &Database,
        settings: &AppSettings,
    ) -> Result<String, String> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/v1/sync/db", settings.server_url);

        let res = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to download DB: {}", e))?;

        if !res.status().is_success() {
            return Err("No release database found on server".to_string());
        }

        let bytes = res.bytes().await.map_err(|e| e.to_string())?;

        // Сохраняем как новую базу
        let app_data = self.file_service.get_base_path_string();
        let db_path = Path::new(&app_data).join("cabinet.db");
        let backup_path = Path::new(&app_data).join("cabinet.db.bak");

        // Бэкап текущей
        if db_path.exists() {
            fs::copy(&db_path, &backup_path).ok();
        }

        // ВАЖНО: SQLite может блокировать файл.
        // Мы пытаемся записать. Если не выйдет — значит база занята.
        // В идеале нужно закрыть соединение, но в Tauri State это сложно.
        // Rusqlite обычно позволяет перезаписать файл, если нет активного обращения.
        fs::write(&db_path, &bytes).map_err(|e| {
            format!(
                "Failed to overwrite DB: {}. Попробуйте перезапустить приложение.",
                e
            )
        })?;

        Ok("Archive updated from server. Please restart to see changes.".to_string())
    }

    // ADMIN: List Releases
    pub async fn list_server_releases(
        &self,
        settings: &AppSettings,
    ) -> Result<Vec<crate::models::ServerRelease>, String> {
        if settings.server_url.is_empty() {
            return Err("Server URL not configured".to_string());
        }

        let client = reqwest::Client::new();
        let url = format!("{}/api/v1/releases", settings.server_url);

        let releases = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", settings.api_key))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?
            .json::<Vec<crate::models::ServerRelease>>()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;

        Ok(releases)
    }

    // ADMIN: Activate Release
    pub async fn activate_server_release(
        &self,
        id: &str,
        settings: &AppSettings,
    ) -> Result<(), String> {
        if settings.server_url.is_empty() {
            return Err("Server URL not configured".to_string());
        }

        let client = reqwest::Client::new();
        let url = format!("{}/api/v1/releases/{}/activate", settings.server_url, id);

        let res = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", settings.api_key))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !res.status().is_success() {
            let txt = res.text().await.unwrap_or_default();
            return Err(format!("Server error: {}", txt));
        }

        Ok(())
    }
}

fn is_external_media_path(path: &str) -> bool {
    path.starts_with("http://") || path.starts_with("https://") || path.starts_with("/static/")
}
