//! SigLIP 2 candle spike — the go/no-go check for visual search ("Хранитель").
//!
//! Loads a FIXED-resolution SigLIP 2 checkpoint (google/siglip2-base-patch16-224)
//! on CPU via candle and prints the cosine similarity between one image and one
//! text query. SigLIP 2 is architecture-compatible with SigLIP v1, and candle's
//! `siglip` module is config.json-driven with the two-tower feature API we need,
//! so this is expected to work (Path A). If it runs and a matching description
//! scores clearly higher than a mismatching one, Path A (candle) is confirmed and
//! we build the visual tower for real; if it can't load the weights, we fall back
//! to Path B (ONNX Runtime via `ort`).
//!
//! Setup (weights are NOT in git):
//!   mkdir -p siglip2-weights && cd siglip2-weights
//!   # from https://huggingface.co/google/siglip2-base-patch16-224/tree/main
//!   #   download: config.json  tokenizer.json  model.safetensors
//!
//! Run:
//!   cd src-tauri/server
//!   cargo run --release --example siglip_spike -- \
//!       ../../siglip2-weights ../../some_figurine.jpg "сгорбленный монах со свечой"
//!
//! Then run it again with a deliberately WRONG description and confirm the score
//! drops. That difference is the whole verdict.

use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use candle_core::{D, DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::siglip;
use image::imageops::FilterType;
use tokenizers::Tokenizer;

fn l2_normalize(v: &Tensor) -> Result<Tensor> {
    let norm = v.sqr()?.sum_keepdim(D::Minus1)?.sqrt()?;
    Ok(v.broadcast_div(&norm)?)
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let weights_dir = PathBuf::from(args.next().context("arg1: weights dir")?);
    let image_path = PathBuf::from(args.next().context("arg2: image path")?);
    let query = args.next().context("arg3: text query")?;

    let device = Device::Cpu;

    // Config straight from the checkpoint's own config.json (nested text/vision).
    let cfg_bytes = std::fs::read(weights_dir.join("config.json")).context("read config.json")?;
    let config: siglip::Config =
        serde_json::from_slice(&cfg_bytes).context("parse SigLIP config.json")?;
    let image_size = config.vision_config.image_size;
    let max_len = config.text_config.max_position_embeddings;
    let pad_id = config.text_config.pad_token_id;
    println!("loaded config: image_size={image_size}, text_max_len={max_len}, pad_id={pad_id}");

    // Model — F32 on CPU, same call shape as depth.rs / embed.rs.
    let vb = unsafe {
        VarBuilder::from_mmaped_safetensors(
            &[weights_dir.join("model.safetensors")],
            DType::F32,
            &device,
        )?
    };
    let model = siglip::Model::new(&config, vb).context("build SigLIP model")?;
    println!("model loaded ✓");

    // ── Image → embedding ────────────────────────────────────────────────────
    let img = image::open(&image_path)
        .with_context(|| format!("open {}", image_path.display()))?
        .to_rgb8();
    let img = image::imageops::resize(
        &img,
        image_size as u32,
        image_size as u32,
        FilterType::Triangle,
    );
    let mut data = vec![0f32; 3 * image_size * image_size];
    let plane = image_size * image_size;
    for (x, y, px) in img.enumerate_pixels() {
        let (x, y) = (x as usize, y as usize);
        for c in 0..3 {
            // SigLIP preprocessing: scale [0,255] → [-1, 1] (affine 2/255, -1).
            data[c * plane + y * image_size + x] = (px[c] as f32) * (2.0 / 255.0) - 1.0;
        }
    }
    let pixel_values = Tensor::from_vec(data, (1, 3, image_size, image_size), &device)?;
    let img_feat = l2_normalize(&model.get_image_features(&pixel_values)?)?;

    // ── Text → embedding ─────────────────────────────────────────────────────
    let tokenizer = Tokenizer::from_file(weights_dir.join("tokenizer.json"))
        .map_err(|e| anyhow!("load tokenizer.json: {e}"))?;
    let enc = tokenizer
        .encode(query.as_str(), true)
        .map_err(|e| anyhow!("tokenize: {e}"))?;
    // SigLIP pads to a FIXED length (max_position_embeddings) with pad_id, no mask.
    let mut ids: Vec<u32> = enc.get_ids().to_vec();
    ids.truncate(max_len);
    if ids.len() < max_len {
        ids.extend(std::iter::repeat(pad_id).take(max_len - ids.len()));
    }
    let input_ids = Tensor::from_vec(ids, (1, max_len), &device)?;
    let txt_feat = l2_normalize(&model.get_text_features(&input_ids)?)?;

    // ── Cosine similarity (both L2-normalised → dot product) ──────────────────
    let sim = img_feat
        .broadcast_mul(&txt_feat)?
        .sum_all()?
        .to_scalar::<f32>()?;
    println!("\nimage: {}", image_path.display());
    println!("query: {query}");
    println!("cosine(image, text) = {sim:.4}");
    println!("→ run again with a WRONG description; a matching one should score clearly higher.");
    Ok(())
}
