use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use tauri::AppHandle;
use tauri::Manager;
use uuid::Uuid;

#[derive(Clone)]
pub struct FileService {
    base_dir: PathBuf,
}

impl FileService {
    pub fn new(app: &AppHandle) -> Self {
        // Получаем путь к AppData
        let base_dir = app
            .path()
            .app_data_dir()
            .expect("Critical: Failed to resolve app data directory");

        // Гарантируем существование директории
        if !base_dir.exists() {
            if let Err(e) = fs::create_dir_all(&base_dir) {
                eprintln!("Critical: Failed to create app data directory: {}", e);
            }
        }

        Self { base_dir }
    }

    /// Получить абсолютный путь к корню данных как строку
    pub fn get_base_path_string(&self) -> String {
        self.base_dir.to_string_lossy().to_string()
    }

    /// Получить полный PathBuf для относительного пути (без проверок безопасности, для внутреннего использования)
    pub fn get_full_path(&self, relative_path: &str) -> PathBuf {
        self.base_dir.join(relative_path)
    }

    /// Безопасно разрешить относительный путь относительно base_dir
    fn resolve_path(&self, relative_path: &str) -> Result<PathBuf, String> {
        let path = Path::new(relative_path);

        // Защита от Path Traversal: проверяем компоненты пути
        for component in path.components() {
            match component {
                Component::Normal(_) => {}
                _ => {
                    return Err(format!(
                        "Invalid path component in '{}': only normal components allowed",
                        relative_path
                    ))
                }
            }
        }

        let dest_path = self.base_dir.join(path);

        // Дополнительная проверка: путь должен начинаться с base_dir
        if !dest_path.starts_with(&self.base_dir) {
            return Err("Access denied: Path is outside of app data directory".to_string());
        }

        Ok(dest_path)
    }

    /// Импортировать внешний файл
    pub fn import_file(&self, source_path: &str, target_subfolder: &str) -> Result<String, String> {
        let target_subfolder = normalize_media_folder(target_subfolder)?;
        let src = Path::new(source_path);
        if !src.exists() {
            return Err(format!("Source file not found: {}", source_path));
        }

        let extension = src
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();

        validate_media_extension(&target_subfolder, &extension)?;

        if target_subfolder == "images" {
            return self.import_image_file(src);
        }

        // Используем UUID для гарантии уникальности
        let new_filename = if extension.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            format!("{}.{}", Uuid::new_v4(), extension)
        };

        // Формируем относительный путь: "videos/uuid.mp4"
        // Используем PathBuf для кросс-платформенного формирования
        let relative_path = Path::new(&target_subfolder)
            .join(&new_filename)
            .to_string_lossy()
            .replace('\\', "/"); // Нормализуем слеши для БД

        let dest_path = self.resolve_path(&relative_path)?;

        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        fs::copy(src, &dest_path).map_err(|e| e.to_string())?;

        Ok(relative_path)
    }

    fn import_image_file(&self, src: &Path) -> Result<String, String> {
        let id = Uuid::new_v4().to_string();
        let original_path = format!("images/original/{}.jpg", id);
        let preview_path = format!("images/preview/{}.jpg", id);
        let thumb_path = format!("images/thumb/{}.jpg", id);

        let image = image::open(src).map_err(|e| format!("Failed to decode image: {}", e))?;

        let original = image.to_rgb8();
        self.save_jpeg(&original_path, &original, 95)?;

        let preview = image.resize(1800, 1800, FilterType::Lanczos3).to_rgb8();
        self.save_jpeg(&preview_path, &preview, 86)?;

        let thumb = image.resize(420, 420, FilterType::Lanczos3).to_rgb8();
        self.save_jpeg(&thumb_path, &thumb, 78)?;

        Ok(preview_path)
    }

    fn save_jpeg(
        &self,
        relative_path: &str,
        image: &image::RgbImage,
        quality: u8,
    ) -> Result<(), String> {
        let dest_path = self.resolve_path(relative_path)?;
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let file = fs::File::create(&dest_path).map_err(|e| e.to_string())?;
        let mut encoder = JpegEncoder::new_with_quality(file, quality);
        encoder.encode_image(image).map_err(|e| e.to_string())
    }

    /// Удалить неиспользуемые медиа-файлы из управляемых папок.
    pub fn cleanup_unused_media(
        &self,
        referenced_paths: &HashSet<String>,
    ) -> Result<Vec<String>, String> {
        let mut removed = Vec::new();
        let preserved = expand_referenced_media_variants(referenced_paths);
        for folder in ["images", "videos", "audio"] {
            let dir = self.resolve_path(folder)?;
            if !dir.exists() {
                continue;
            }

            self.cleanup_dir_recursive(&dir, &preserved, &mut removed)?;
        }

        Ok(removed)
    }

    pub fn list_managed_media_files(&self) -> Result<Vec<(String, u64)>, String> {
        let mut files = Vec::new();
        for folder in ["images", "videos", "audio"] {
            let dir = self.resolve_path(folder)?;
            if dir.exists() {
                self.collect_files_recursive(&dir, &mut files)?;
            }
        }
        files.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(files)
    }

    pub fn media_file_size(&self, relative_path: &str) -> Option<u64> {
        self.resolve_path(relative_path)
            .ok()
            .and_then(|path| fs::metadata(path).ok())
            .map(|meta| meta.len())
    }

    pub fn delete_media_files(&self, relative_paths: &[String]) -> Result<Vec<String>, String> {
        let mut removed = Vec::new();
        for path in relative_paths {
            let full_path = self.resolve_path(path)?;
            if full_path.exists() && full_path.is_file() {
                fs::remove_file(&full_path).map_err(|e| e.to_string())?;
                removed.push(path.clone());
            }
        }
        Ok(removed)
    }

    fn collect_files_recursive(
        &self,
        dir: &Path,
        files: &mut Vec<(String, u64)>,
    ) -> Result<(), String> {
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                self.collect_files_recursive(&path, files)?;
                continue;
            }
            if !path.is_file() {
                continue;
            }
            let rel = path
                .strip_prefix(&self.base_dir)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            let size = fs::metadata(&path).map_err(|e| e.to_string())?.len();
            files.push((rel, size));
        }
        Ok(())
    }

    fn cleanup_dir_recursive(
        &self,
        dir: &Path,
        preserved_paths: &HashSet<String>,
        removed: &mut Vec<String>,
    ) -> Result<(), String> {
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();

            if path.is_dir() {
                self.cleanup_dir_recursive(&path, preserved_paths, removed)?;
                if fs::read_dir(&path)
                    .map_err(|e| e.to_string())?
                    .next()
                    .is_none()
                {
                    fs::remove_dir(&path).map_err(|e| e.to_string())?;
                }
                continue;
            }

            if !path.is_file() {
                continue;
            }

            let rel = path
                .strip_prefix(&self.base_dir)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/");

            if !preserved_paths.contains(&rel) {
                fs::remove_file(&path).map_err(|e| e.to_string())?;
                removed.push(rel);
            }
        }

        Ok(())
    }

    /// Проверить существование файла
    pub fn file_exists(&self, relative_path: &str) -> bool {
        if let Ok(path) = self.resolve_path(relative_path) {
            path.exists()
        } else {
            false
        }
    }

    /// Очищает путь от asset://, cabinet:// или абсолютного пути AppData
    pub fn clean_path(&self, full_path: &str) -> String {
        let mut path = full_path.to_string();

        // 1. Remove protocols
        if path.starts_with("cabinet://") {
            path = path
                .replace("cabinet://localhost", "")
                .replace("cabinet://", "");
        } else if path.starts_with("asset://") {
            path = path
                .replace("asset://localhost", "")
                .replace("asset://", "");
        }

        // 2. Remove base dir if present (absolute path case)
        let base_str = self.base_dir.to_string_lossy().to_string();
        // Normalize slashes for comparison just in case
        let path_norm = path.replace('\\', "/");
        let base_norm = base_str.replace('\\', "/");

        if path_norm.contains(&base_norm) {
            path = path_norm.replace(&base_norm, "");
        } else if path.contains(&base_str) {
            path = path.replace(&base_str, "");
        }

        // 3. Trim leading slashes
        path.trim_start_matches('/')
            .trim_start_matches('\\')
            .to_string()
    }
}

fn normalize_media_folder(folder: &str) -> Result<String, String> {
    match folder {
        "images" | "videos" | "audio" => Ok(folder.to_string()),
        _ => Err(format!("Unsupported media folder: {}", folder)),
    }
}

fn validate_media_extension(folder: &str, extension: &str) -> Result<(), String> {
    let is_valid = match folder {
        "images" => matches!(extension, "jpg" | "jpeg" | "png" | "webp"),
        "videos" => matches!(extension, "mp4" | "webm" | "mov"),
        "audio" => matches!(extension, "mp3" | "wav" | "ogg" | "m4a"),
        _ => false,
    };

    if is_valid {
        Ok(())
    } else {
        Err(format!(
            "Unsupported {} file extension: {}",
            folder, extension
        ))
    }
}

fn expand_referenced_media_variants(referenced_paths: &HashSet<String>) -> HashSet<String> {
    let mut preserved = referenced_paths.clone();

    for path in referenced_paths {
        if let Some(file_name) = path.strip_prefix("images/preview/") {
            preserved.insert(format!("images/original/{}", file_name));
            preserved.insert(format!("images/thumb/{}", file_name));
        }
    }

    preserved
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_media_extensions_by_folder() {
        assert!(validate_media_extension("images", "jpg").is_ok());
        assert!(validate_media_extension("images", "webp").is_ok());
        assert!(validate_media_extension("videos", "mp4").is_ok());
        assert!(validate_media_extension("audio", "mp3").is_ok());

        assert!(validate_media_extension("images", "mp4").is_err());
        assert!(validate_media_extension("videos", "jpg").is_err());
        assert!(validate_media_extension("audio", "png").is_err());
    }

    #[test]
    fn expands_preview_references_to_image_variants() {
        let mut referenced = HashSet::new();
        referenced.insert("images/preview/abc.jpg".to_string());
        referenced.insert("videos/clip.mp4".to_string());

        let expanded = expand_referenced_media_variants(&referenced);

        assert!(expanded.contains("images/preview/abc.jpg"));
        assert!(expanded.contains("images/original/abc.jpg"));
        assert!(expanded.contains("images/thumb/abc.jpg"));
        assert!(expanded.contains("videos/clip.mp4"));
    }
}
