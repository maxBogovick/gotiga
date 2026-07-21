//! On-device text embeddings for semantic search ("Хранитель").
//!
//! Runs a multilingual sentence bi-encoder on CPU via candle — pure Rust,
//! in-process, no Python and no external API (exactly like `depth.rs`). The
//! model loads lazily on first use and is kept warm behind a Mutex that also
//! serialises inference, bounding peak RAM on a small box.
//!
//! Default model: **intfloat/multilingual-e5-small** — an XLM-RoBERTa bi-encoder
//! covering ~100 languages (so a Russian *or* English query works against the
//! same vectors), 384-dim, ~118M params, mean-pooled + L2-normalised. It is the
//! size/quality/latency sweet spot for on-device retrieval over a tiny museum
//! corpus. The choice is not baked in: point `EMBED_WEIGHTS_DIR` at any other
//! XLM-RoBERTa checkpoint (bge-m3, a newer multilingual-e5, …) with its own
//! `config.json` + `tokenizer.json` + `model.safetensors` and the loader adapts;
//! the DB `model` column keeps old vectors from being compared across models.
//!
//! e5 requires input prefixes: queries as `query: …`, documents as `passage: …`.
//! Both are applied here so callers pass plain text.

use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::{Context, Result, anyhow};
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::xlm_roberta::{Config, XLMRobertaModel};
use tokenizers::Tokenizer;

/// Model identity persisted alongside every vector (see the `model` column).
/// Overridable so a redeploy on a different checkpoint doesn't silently mix
/// incompatible vectors.
pub fn model_name() -> String {
    std::env::var("EMBED_MODEL_NAME").unwrap_or_else(|_| "intfloat/multilingual-e5-small".into())
}

/// Longest input we tokenise. Curatorial blurbs are short; 256 tokens is plenty
/// and keeps a single CPU forward pass in the low-hundreds-of-ms range.
const MAX_TOKENS: usize = 256;

fn weights_dir() -> PathBuf {
    std::env::var("EMBED_WEIGHTS_DIR")
        .unwrap_or_else(|_| "/app/embed-weights".to_string())
        .into()
}
fn model_file() -> PathBuf {
    weights_dir().join("model.safetensors")
}
fn tokenizer_file() -> PathBuf {
    weights_dir().join("tokenizer.json")
}
fn config_file() -> PathBuf {
    weights_dir().join("config.json")
}

/// Whether embedding is available (all three model files are present). Callers
/// use this to degrade gracefully — no weights bundled → search/index no-op.
pub fn is_available() -> bool {
    model_file().exists() && tokenizer_file().exists() && config_file().exists()
}

struct Model {
    inner: XLMRobertaModel,
    tokenizer: Tokenizer,
    device: Device,
    dim: usize,
}

// SAFETY: mirrors depth.rs. The model is only ever touched while MODEL's mutex
// is held (one thread at a time — see `embed`). Nothing is accessed concurrently.
unsafe impl Send for Model {}

static MODEL: Mutex<Option<Model>> = Mutex::new(None);

fn load_model() -> Result<Model> {
    let device = Device::Cpu;

    // Config comes straight from the checkpoint's own config.json, so a different
    // XLM-RoBERTa model just works without editing this file.
    let cfg_bytes = std::fs::read(config_file())
        .with_context(|| format!("read {}", config_file().display()))?;
    let config: Config =
        serde_json::from_slice(&cfg_bytes).context("parse embedding model config.json")?;
    let dim = config.hidden_size;

    let mut tokenizer = Tokenizer::from_file(tokenizer_file())
        .map_err(|e| anyhow!("load tokenizer.json: {e}"))?;
    // Fixed-length truncation; padding is unnecessary for single-sequence encoding.
    let truncation = tokenizers::TruncationParams {
        max_length: MAX_TOKENS,
        ..Default::default()
    };
    let _ = tokenizer.with_truncation(Some(truncation));

    // F32 on CPU (F16 CPU kernels are slow/partial) — same call shape as depth.rs.
    let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[model_file()], DType::F32, &device)? };
    let inner = XLMRobertaModel::new(&config, vb).context("build XLM-RoBERTa encoder")?;

    Ok(Model {
        inner,
        tokenizer,
        device,
        dim,
    })
}

/// Encode one already-prefixed string to an L2-normalised embedding.
/// Blocking / CPU-bound — call from a blocking context (spawn_blocking).
fn embed(text: &str) -> Result<Vec<f32>> {
    let mut guard = MODEL.lock().unwrap();
    if guard.is_none() {
        if !is_available() {
            return Err(anyhow!(
                "embedding weights not found in {}",
                weights_dir().display()
            ));
        }
        *guard = Some(load_model()?);
    }
    let m = guard.as_ref().unwrap();

    let encoding = m
        .tokenizer
        .encode(text, true)
        .map_err(|e| anyhow!("tokenize: {e}"))?;
    let ids: Vec<u32> = encoding.get_ids().to_vec();
    let mask_u32: Vec<u32> = encoding.get_attention_mask().to_vec();
    let seq = ids.len();

    // input_ids / token_type_ids feed embedding lookups → U32 indices.
    let input_ids = Tensor::from_vec(ids, (1, seq), &m.device)?;
    // XLM-RoBERTa is single-segment here → all token-type ids are 0.
    let token_type_ids = Tensor::zeros((1, seq), DType::U32, &m.device)?;
    // The model turns this into an additive mask (prepare_4d_attention_mask does
    // `1.0 - mask`), so a float 0/1 mask is the dtype-safe input.
    let mask_f: Vec<f32> = mask_u32.iter().map(|&x| x as f32).collect();
    let attention_mask = Tensor::from_vec(mask_f, (1, seq), &m.device)?;

    // (batch, seq, hidden) last hidden state.
    let hidden = m
        .inner
        .forward(&input_ids, &attention_mask, &token_type_ids, None, None, None)?;

    // Attention-masked mean pooling → (batch, hidden). attention_mask is F32 0/1.
    let mask_exp = attention_mask.unsqueeze(2)?; // (1, seq, 1)
    let summed = hidden.broadcast_mul(&mask_exp)?.sum(1)?; // (1, hidden)
    let counts = attention_mask.sum(1)?.clamp(1e-9f32, f32::MAX)?.unsqueeze(1)?; // (1, 1)
    let mean = summed.broadcast_div(&counts)?; // (1, hidden)

    // L2 normalise so cosine similarity is a plain dot product downstream.
    let norm = mean.sqr()?.sum_keepdim(1)?.sqrt()?; // (1, 1)
    let normalized = mean.broadcast_div(&norm)?; // (1, hidden)

    let mut v = normalized.flatten_all()?.to_vec1::<f32>()?;
    debug_assert_eq!(v.len(), m.dim);
    // Guard against a NaN row (empty text) poisoning later dot products.
    if v.iter().any(|x| !x.is_finite()) {
        v = vec![0.0; m.dim];
    }
    Ok(v)
}

/// Embed a natural-language search query (e5 `query:` prefix).
pub fn embed_query(q: &str) -> Result<Vec<f32>> {
    embed(&format!("query: {}", q.trim()))
}

/// Embed a work's curatorial text as a searchable document (e5 `passage:` prefix).
pub fn embed_passage(text: &str) -> Result<Vec<f32>> {
    embed(&format!("passage: {}", text.trim()))
}

/// Dot product of two equal-length vectors. For L2-normalised inputs this equals
/// cosine similarity.
pub fn dot(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return f32::NEG_INFINITY;
    }
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Pack an embedding to the on-disk BYTEA layout (little-endian f32).
pub fn to_bytes(v: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(v.len() * 4);
    for x in v {
        out.extend_from_slice(&x.to_le_bytes());
    }
    out
}

/// Unpack a stored BYTEA embedding back to f32s. Returns None on a truncated blob.
pub fn from_bytes(bytes: &[u8]) -> Option<Vec<f32>> {
    if bytes.len() % 4 != 0 {
        return None;
    }
    Some(
        bytes
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect(),
    )
}
