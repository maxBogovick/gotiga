use crate::config::Config;
use crate::db::Repository;
use crate::error::Result;
use crate::models::*;
use uuid::Uuid;
use chrono::Utc;

#[derive(Clone)]
pub struct AppService {
    repo: Repository,
    config: Config,
}

impl AppService {
    pub fn new(repo: Repository, config: Config) -> Self {
        Self { repo, config }
    }

    // Helper to format full URL
    fn to_full_url(&self, relative_path: &str) -> String {
        if relative_path.starts_with("http") {
            relative_path.to_string()
        } else {
            // Ensure no double slash
            let base = self.config.public_url.trim_end_matches('/');
            let path = relative_path.trim_start_matches('/');
            format!("{}/static/{}", base, path)
        }
    }

    pub async fn list_figurines(&self, visible_only: bool) -> Result<Vec<FigurineListItemDto>> {
        let figurines = self.repo.get_all_figurines(visible_only).await?;
        let mut result = Vec::new();

        for f in figurines {
            // Optimization: We could join in SQL, but for now simple N+1 is acceptable for small datasets
            // or we could fetch all images once and map in memory.
            let images = self.repo.get_images_by_figurine(f.id).await?;
            let face_img = images.iter()
                .find(|i| i.image_type == ImageType::Face)
                .map(|i| self.to_full_url(&i.file_path));

            result.push(FigurineListItemDto {
                id: f.id,
                name: f.name,
                status: f.status,
                face_image_url: face_img,
            });
        }
        Ok(result)
    }

    pub async fn get_figurine_details(&self, id: Uuid) -> Result<FigurineDto> {
        let figurine = self.repo.get_figurine_by_id(id).await?
            .ok_or_else(|| crate::error::AppError::NotFound(format!("Figurine {} not found", id)))?;

        let images = self.repo.get_images_by_figurine(id).await?;
        let steps = self.repo.get_steps_by_figurine(id).await?;
        let related_entities = self.repo.get_related_figurines(id).await?;

        // Map Related Items
        let mut related_items = Vec::new();
        for r in related_entities {
             let r_imgs = self.repo.get_images_by_figurine(r.id).await?;
             let face = r_imgs.iter()
                .find(|i| i.image_type == ImageType::Face)
                .map(|i| self.to_full_url(&i.file_path));

            related_items.push(FigurineListItemDto {
                id: r.id,
                name: r.name,
                status: r.status,
                face_image_url: face,
            });
        }

        // Map Images
        let image_dtos = images.into_iter().map(|i| ImageDto {
            id: Some(i.id),
            image_type: i.image_type,
            url: self.to_full_url(&i.file_path),
            alt_text: i.alt_text,
        }).collect();

        // Map Steps
        let step_dtos = steps.into_iter().map(|s| ProcessStepDto {
            id: Some(s.id),
            step_type: s.step_type,
            description: s.description,
            image_url: self.to_full_url(&s.image_path),
        }).collect();

        Ok(FigurineDto {
            id: Some(figurine.id),
            name: figurine.name,
            short_text: figurine.short_text,
            full_description: figurine.full_description,
            dimensions: figurine.dimensions,
            material: figurine.material,
            technique: figurine.technique,
            year: figurine.year,
            ambience_path: figurine.ambience_path.map(|p| self.to_full_url(&p)),
            video_url: figurine.video_url.map(|p| self.to_full_url(&p)),
            secret_text: figurine.secret_text,
            status: figurine.status,
            sort_order: figurine.sort_order,
            is_visible: figurine.is_visible,
            images: image_dtos,
            process_steps: step_dtos,
            related_items,
        })
    }

    pub async fn upsert_figurine(&self, dto: FigurineDto) -> Result<Uuid> {
        let id = dto.id.unwrap_or_else(Uuid::new_v4);

        // Helper to strip public URL back to relative path
        let clean_path = |full_url: String| -> String {
            if full_url.contains("/static/") {
                 full_url.split("/static/").nth(1).unwrap_or(&full_url).to_string()
            } else {
                full_url
            }
        };

        // 1. Upsert Parent
        let entity = Figurine {
            id,
            name: dto.name,
            short_text: dto.short_text,
            full_description: dto.full_description,
            dimensions: dto.dimensions,
            material: dto.material,
            technique: dto.technique,
            year: dto.year,
            ambience_path: dto.ambience_path.map(|p| clean_path(p)),
            video_url: dto.video_url.map(|p| clean_path(p)),
            secret_text: dto.secret_text,
            is_visible: dto.is_visible,
            status: dto.status,
            sort_order: dto.sort_order,
            created_at: Utc::now(), // Repo ignores this on update
            updated_at: Utc::now(),
        };

        self.repo.upsert_figurine(&entity).await?;

        // 2. Prepare Children (Images)
        let images: Vec<Image> = dto.images.into_iter().enumerate().map(|(idx, img_dto)| {
            Image {
                id: img_dto.id.unwrap_or_else(Uuid::new_v4),
                figurine_id: id,
                image_type: img_dto.image_type,
                file_path: clean_path(img_dto.url),
                alt_text: img_dto.alt_text,
                sort_order: idx as i32,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
        }).collect();

        self.repo.replace_images(id, images).await?;

        // 3. Prepare Children (Steps)
        let steps: Vec<ProcessStep> = dto.process_steps.into_iter().enumerate().map(|(idx, step_dto)| {
            ProcessStep {
                id: step_dto.id.unwrap_or_else(Uuid::new_v4),
                figurine_id: id,
                step_type: step_dto.step_type,
                description: step_dto.description,
                image_path: clean_path(step_dto.image_url),
                sort_order: idx as i32,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
        }).collect();

        self.repo.replace_steps(id, steps).await?;

        Ok(id)
    }

    pub async fn delete_figurine(&self, id: Uuid) -> Result<()> {
        self.repo.delete_figurine(id).await
    }

    // === MANIFEST GENERATION (Backward Compat) ===
    pub async fn generate_manifest(&self) -> Result<Manifest> {
        let figurines = self.repo.get_all_figurines(false).await?;
        let images = self.repo.get_all_images().await?;
        let process_steps = self.repo.get_all_steps().await?;

        Ok(Manifest {
            version: Utc::now().timestamp(),
            generated_at: Utc::now().to_rfc3339(),
            figurines,
            images,
            process_steps,
        })
    }

    // === CONTENT ===
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
            id: t.id,
            content: t.content,
            caption: t.caption,
            image_url: t.image_path.map(|p| self.to_full_url(&p)),
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

    pub async fn process_full_release(&self, payload: ReleasePayload) -> Result<()> {
        let mut figurines = Vec::new();
        let mut images = Vec::new();
        let mut steps = Vec::new();
        let mut texts = Vec::new();
        let mut zones = Vec::new();

        // Helper to strip public URL back to relative path
        let clean_path = |full_url: String| -> String {
            if full_url.contains("/static/") {
                 full_url.split("/static/").nth(1).unwrap_or(&full_url).to_string()
            } else {
                full_url
            }
        };
        let clean_path_opt = |full_url: Option<String>| -> Option<String> {
             full_url.map(|u| clean_path(u))
        };

        // 1. Figurines & their children
        for f_dto in payload.figurines {
            let f_id = f_dto.id.unwrap_or_else(Uuid::new_v4);
            
            figurines.push(Figurine {
                id: f_id,
                name: f_dto.name,
                short_text: f_dto.short_text,
                full_description: f_dto.full_description,
                dimensions: f_dto.dimensions,
                material: f_dto.material,
                technique: f_dto.technique,
                year: f_dto.year,
                ambience_path: clean_path_opt(f_dto.ambience_path),
                video_url: clean_path_opt(f_dto.video_url),
                secret_text: f_dto.secret_text,
                is_visible: f_dto.is_visible,
                status: f_dto.status,
                sort_order: f_dto.sort_order,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            });

            for (idx, img_dto) in f_dto.images.into_iter().enumerate() {
                images.push(Image {
                    id: img_dto.id.unwrap_or_else(Uuid::new_v4),
                    figurine_id: f_id,
                    image_type: img_dto.image_type,
                    file_path: clean_path(img_dto.url),
                    alt_text: img_dto.alt_text,
                    sort_order: idx as i32,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                });
            }

            for (idx, step_dto) in f_dto.process_steps.into_iter().enumerate() {
                steps.push(ProcessStep {
                    id: step_dto.id.unwrap_or_else(Uuid::new_v4),
                    figurine_id: f_id,
                    step_type: step_dto.step_type,
                    description: step_dto.description,
                    image_path: clean_path(step_dto.image_url),
                    sort_order: idx as i32,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                });
            }
        }

        // 2. Texts
        for (idx, t_dto) in payload.author_texts.into_iter().enumerate() {
            texts.push(Text {
                id: t_dto.id,
                category: TextCategory::Author,
                content: t_dto.content,
                caption: None,
                image_path: None,
                sort_order: idx as i32,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            });
        }

        for (idx, w_dto) in payload.workshop_items.into_iter().enumerate() {
            texts.push(Text {
                id: w_dto.id,
                category: TextCategory::Workshop,
                content: w_dto.content,
                caption: w_dto.caption,
                image_path: clean_path_opt(w_dto.image_url),
                sort_order: idx as i32,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            });
        }

        // 3. Zones
        for (idx, z_dto) in payload.zones.into_iter().enumerate() {
            zones.push(CabinetZone {
                id: z_dto.id,
                zone_type: z_dto.zone_type,
                x_percent: z_dto.x,
                y_percent: z_dto.y,
                width_percent: z_dto.width,
                height_percent: z_dto.height,
                target_route: z_dto.target_route,
                sort_order: idx as i32,
            });
        }

        self.repo.replace_full_state(figurines, images, steps, texts, zones).await
    }
}
