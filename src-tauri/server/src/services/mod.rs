use crate::config::Config;
use crate::db::Repository;
use crate::error::Result;
use crate::models::*;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppService {
    repo: Repository,
    config: Config,
}

impl AppService {
    pub fn new(repo: Repository, config: Config) -> Self {
        Self { repo, config }
    }

    pub async fn initialize(&self) -> Result<()> {
        if let Some(path) = self.repo.get_active_release_path().await? {
            println!("Initializing content pool from: {}", path);
            if std::path::Path::new(&path).exists() {
                self.repo.load_content_pool(&path).await?;
            } else {
                eprintln!("Warning: Active release file not found at {}", path);
            }
        }
        Ok(())
    }

    // === RELEASE MANAGEMENT ===

    pub async fn register_new_release(&self, file_path: &str) -> Result<()> {
        let id = self.repo.add_release(file_path, None).await?;
        self.repo.activate_release(id).await?;
        self.repo.load_content_pool(file_path).await?;
        println!("Switched to new release: {}", file_path);
        Ok(())
    }

    pub async fn list_releases(&self) -> Result<Vec<Release>> {
        self.repo.get_releases().await
    }

    pub async fn switch_to_release(&self, id: Uuid) -> Result<()> {
        let release = self.repo.get_release_by_id(id).await?
            .ok_or_else(|| crate::error::AppError::NotFound(format!("Release {} not found", id)))?;
            
        self.repo.activate_release(id).await?;
        self.repo.load_content_pool(&release.file_path).await?;
        println!("Rolled back/Switched to release: {}", release.file_path);
        Ok(())
    }

    pub async fn get_active_release_path(&self) -> Result<Option<String>> {
        self.repo.get_active_release_path().await
    }

    // === CONTENT API (READ-ONLY from SQLite) ===

    fn asset_url(&self, table: &str, id: &str) -> String {
        let base = self.config.public_url.trim_end_matches('/');
        format!("{}/api/v1/assets/{}/{}", base, table, id)
    }

    // Use external URL if already http, otherwise serve via asset endpoint
    fn resolve_url(&self, file_path: &str, table: &str, id: &str) -> String {
        if file_path.starts_with("http") {
            file_path.to_string()
        } else {
            self.asset_url(table, id)
        }
    }

    pub async fn list_figurines(&self, visible_only: bool) -> Result<Vec<FigurineListItemDto>> {
        let figurines = self.repo.get_all_figurines(visible_only).await?;
        let mut result = Vec::new();

        for f in figurines {
            let images = self.repo.get_images_by_figurine(f.id.clone()).await?;
            let face_img = images.iter()
                .find(|i| i.image_type == ImageType::Face)
                .map(|i| self.resolve_url(&i.file_path, "images", &i.id));

            result.push(FigurineListItemDto {
                id: f.id,
                name: f.name,
                status: f.status,
                face_image_url: face_img,
            });
        }
        Ok(result)
    }

    pub async fn get_figurine_details(&self, id: String) -> Result<FigurineDto> {
        let figurine = self.repo.get_figurine_by_id(id.clone()).await?
            .ok_or_else(|| crate::error::AppError::NotFound(format!("Figurine {} not found", id)))?;

        let images = self.repo.get_images_by_figurine(id.clone()).await?;
        let steps = self.repo.get_steps_by_figurine(id.clone()).await?;
        let related_entities = self.repo.get_related_figurines(id.clone()).await?;

        let mut related_items = Vec::new();
        for r in related_entities {
            let r_imgs = self.repo.get_images_by_figurine(r.id.clone()).await?;
            let face = r_imgs.iter()
                .find(|i| i.image_type == ImageType::Face)
                .map(|i| self.resolve_url(&i.file_path, "images", &i.id));

            related_items.push(FigurineListItemDto {
                id: r.id,
                name: r.name,
                status: r.status,
                face_image_url: face,
            });
        }

        let image_dtos = images.into_iter().map(|i| ImageDto {
            id: i.id.clone(),
            image_type: i.image_type,
            url: self.resolve_url(&i.file_path, "images", &i.id),
            alt_text: i.alt_text,
        }).collect();

        let step_dtos = steps.into_iter().map(|s| ProcessStepDto {
            id: s.id.clone(),
            step_type: s.step_type,
            description: s.description,
            image_url: self.resolve_url(&s.image_path, "process_steps", &s.id),
        }).collect();

        Ok(FigurineDto {
            id: figurine.id.clone(),
            name: figurine.name,
            short_text: figurine.short_text,
            full_description: figurine.full_description,
            dimensions: figurine.dimensions,
            material: figurine.material,
            technique: figurine.technique,
            year: figurine.year,
            ambience_path: figurine.ambience_path.as_ref()
                .map(|p| self.resolve_url(p, "figurines_audio", &figurine.id)),
            video_url: figurine.video_url.as_ref()
                .map(|p| self.resolve_url(p, "figurines_video", &figurine.id)),
            secret_text: figurine.secret_text,
            status: figurine.status,
            sort_order: figurine.sort_order,
            is_visible: figurine.is_visible,
            images: image_dtos,
            process_steps: step_dtos,
            related_items,
        })
    }

    pub async fn get_author_texts(&self) -> Result<Vec<TextDto>> {
        let texts = self.repo.get_texts_by_category(TextCategory::Author).await?;
        Ok(texts.into_iter().map(|t| TextDto {
            id: t.id,
            content: t.content
        }).collect())
    }

    pub async fn get_workshop_items(&self) -> Result<Vec<WorkshopItemDto>> {
        let texts = self.repo.get_texts_by_category(TextCategory::Workshop).await?;
        Ok(texts.into_iter().map(|t| WorkshopItemDto {
            id: t.id.clone(),
            content: t.content,
            caption: t.caption,
            image_url: t.image_path.as_ref().map(|p| self.resolve_url(p, "texts", &t.id)),
        }).collect())
    }

    pub async fn get_cabinet_zones(&self) -> Result<Vec<CabinetZoneDto>> {
        let zones = self.repo.get_zones().await?;
        Ok(zones.into_iter().map(|z| CabinetZoneDto {
            id: z.id,
            zone_type: z.zone_type,
            x: z.x_percent,
            y: z.y_percent,
            width: z.width_percent,
            height: z.height_percent,
            target_route: z.target_route,
        }).collect())
    }

    pub async fn get_asset(&self, table: &str, id: String) -> Result<Option<Vec<u8>>> {
        let (real_table, column) = match table {
            "images" => ("images", "data"),
            "process_steps" => ("process_steps", "image_data"),
            "figurines_video" => ("figurines", "video_data"),
            "figurines_audio" => ("figurines", "ambience_data"),
            "texts" => ("texts", "image_data"),
            _ => return Err(crate::error::AppError::BadRequest("Invalid asset type".to_string())),
        };
        
        self.repo.get_blob(real_table, column, id).await
    }
}
