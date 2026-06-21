//! On-demand monocular depth maps for the "Living Daguerreotype" 2.5D parallax.
//!
//! Runs Depth-Anything-V2 (small) on CPU via candle — pure Rust, in-process, no
//! Python and no torch image. The model loads lazily on the first request and is
//! kept warm; one generation runs at a time (GEN_LOCK) to bound peak memory on a
//! small box. Weights are bundled into the image at build time (see Dockerfile).
//!
//! Architecture mirrors candle's own depth_anything_v2 example: a DINOv2-small
//! backbone (lmz/candle-dino-v2 · dinov2_vits14.safetensors) feeds the
//! Depth-Anything head (jeroenvlek/depth-anything-v2-safetensors ·
//! depth_anything_v2_vits.safetensors). Two safetensors, two VarBuilders.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, anyhow};
use candle_core::{DType, Device, Module, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::depth_anything_v2::{DepthAnythingV2, DepthAnythingV2Config};
use candle_transformers::models::dinov2;
use image::imageops::FilterType;

const IMG_SIZE: usize = 518; // DINOv2 patch grid: 518 = 14 × 37
const MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const STD: [f32; 3] = [0.229, 0.224, 0.225];

fn weights_dir() -> PathBuf {
    std::env::var("DEPTH_WEIGHTS_DIR")
        .unwrap_or_else(|_| "/app/depth-weights".to_string())
        .into()
}
fn dino_file() -> PathBuf {
    weights_dir().join("dinov2_vits14.safetensors")
}
fn head_file() -> PathBuf {
    weights_dir().join("depth_anything_v2_vits.safetensors")
}

/// Whether depth generation is available (both weight files are present).
pub fn is_available() -> bool {
    dino_file().exists() && head_file().exists()
}

/// Resolve a stored image path (full public URL, '/static/…' or relative) to its
/// file on disk under `upload_dir`. The API serves '/static/images' from
/// '<upload_dir>/images', so we keep everything after '/static/'.
pub fn local_source(stored: &str, upload_dir: &str) -> PathBuf {
    let p = stored.split(['?', '#']).next().unwrap_or(stored);
    let rel = if let Some(idx) = p.find("/static/") {
        &p[idx + "/static/".len()..]
    } else if p.contains("://") {
        p.splitn(4, '/').nth(3).unwrap_or(p) // strip scheme://host/
    } else {
        p
    };
    PathBuf::from(upload_dir).join(rel.trim_start_matches('/'))
}

struct Model {
    inner: DepthAnythingV2,
    device: Device,
}

// SAFETY: the model is only ever touched while MODEL's mutex is held (one thread
// at a time — see `generate`). Its lack of an auto `Send` impl comes solely from
// the `Box<dyn Module>` trait objects inside DPTHead, whose actual data is candle
// Tensors (Send). Nothing is accessed concurrently, so moving it between threads
// is sound.
unsafe impl Send for Model {}

// Loaded once, kept warm. Held behind a single Mutex that also serialises
// inference — one generation at a time keeps peak RAM bounded on a small box.
static MODEL: Mutex<Option<Model>> = Mutex::new(None);

fn load_model() -> Result<Model> {
    let device = Device::Cpu;
    let vb_dino =
        unsafe { VarBuilder::from_mmaped_safetensors(&[dino_file()], DType::F32, &device)? };
    let dino = dinov2::vit_small(vb_dino).context("load DINOv2 backbone")?;
    let vb_head =
        unsafe { VarBuilder::from_mmaped_safetensors(&[head_file()], DType::F32, &device)? };
    let inner = DepthAnythingV2::new(Arc::new(dino), DepthAnythingV2Config::vit_small(), vb_head)
        .context("build Depth-Anything head")?;
    Ok(Model { inner, device })
}

/// Generate a grayscale depth map for `src` and write it to `out` (PNG, sized to
/// the source image so the shader's UVs line up). Blocking/CPU-bound — call from
/// a blocking context. Convention: brighter = nearer (matches the shader).
pub fn generate(src: &Path, out: &Path) -> Result<()> {
    // Lock for the whole call: lazily loads the model and serialises inference.
    let mut guard = MODEL.lock().unwrap();
    if guard.is_none() {
        if !is_available() {
            return Err(anyhow!(
                "depth weights not found in {}",
                weights_dir().display()
            ));
        }
        *guard = Some(load_model()?);
    }
    let m = guard.as_ref().unwrap();

    let rgb = image::open(src)
        .with_context(|| format!("open {}", src.display()))?
        .to_rgb8();
    let (ow, oh) = (rgb.width(), rgb.height());
    let resized = image::imageops::resize(
        &rgb,
        IMG_SIZE as u32,
        IMG_SIZE as u32,
        FilterType::CatmullRom,
    );

    // CHW, /255, ImageNet-normalised.
    let mut data = vec![0f32; 3 * IMG_SIZE * IMG_SIZE];
    let plane = IMG_SIZE * IMG_SIZE;
    for (x, y, px) in resized.enumerate_pixels() {
        let (x, y) = (x as usize, y as usize);
        for c in 0..3 {
            data[c * plane + y * IMG_SIZE + x] = (px[c] as f32 / 255.0 - MEAN[c]) / STD[c];
        }
    }
    let input = Tensor::from_vec(data, (1, 3, IMG_SIZE, IMG_SIZE), &m.device)?;

    let depth = m.inner.forward(&input)?;
    let dims = depth.dims().to_vec();
    let (dh, dw) = (dims[dims.len() - 2], dims[dims.len() - 1]);
    let flat = depth
        .flatten_all()?
        .to_dtype(DType::F32)?
        .to_vec1::<f32>()?;

    // Min-max stretch to 0..255 (so the shader's [0,1] range is fully used).
    let (mut mn, mut mx) = (f32::INFINITY, f32::NEG_INFINITY);
    for &v in &flat {
        if v < mn {
            mn = v;
        }
        if v > mx {
            mx = v;
        }
    }
    let span = if (mx - mn).abs() < 1e-6 { 1.0 } else { mx - mn };

    let mut gray = image::GrayImage::new(dw as u32, dh as u32);
    for (i, px) in gray.pixels_mut().enumerate() {
        let v = (((flat[i] - mn) / span).clamp(0.0, 1.0) * 255.0) as u8;
        *px = image::Luma([v]);
    }
    // Resize the depth to the colour image's grid so the parallax aligns exactly.
    let gray = image::imageops::resize(&gray, ow, oh, FilterType::Triangle);

    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    gray.save(out)
        .with_context(|| format!("save {}", out.display()))?;
    Ok(())
}
