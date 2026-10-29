//! Проба для поиска по фото: может ли сервер сам превращать запрос посетителя в вектор
//! SigLIP 2 (только текстовая половина модели), и сколько это стоит.
//!
//! Загружает из `model.safetensors` ровно текстовую башню (`text_model.*`), токенизирует
//! запросы, считает векторы и сверяет их с теми, что посчитал питон
//! (`tools/embed/models.py`, файл с эталоном делает скрипт `ref_siglip.py`).
//! Печатает: совпали ли id токенов, косинус между векторами сервера и питона,
//! память процесса после загрузки и время одного запроса.
//!
//!   cargo run --release --example siglip_text_probe -- \
//!       <каталог с config.json, tokenizer.json, model.safetensors> \
//!       <ref_siglip.json> <имя модели в эталоне> [f32|bf16|lite]
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use anyhow::{Context, Result, anyhow};
use candle_core::{D, DType, Device, Tensor};
use candle_core::safetensors::MmapedSafetensors;
use candle_nn::{LayerNorm, Linear, Module, VarBuilder, layer_norm, linear};
use candle_transformers::models::siglip;
use serde_json::Value;
use tokenizers::Tokenizer;

fn rss_mb() -> f64 {
    let pid = std::process::id().to_string();
    let out = Command::new("ps").args(["-o", "rss=", "-p", &pid]).output().ok();
    out.and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<f64>().ok())
        .map(|kb| kb / 1024.0)
        .unwrap_or(0.0)
}

fn l2(v: &Tensor) -> Result<Tensor> {
    let n = v.sqr()?.sum_keepdim(D::Minus1)?.sqrt()?;
    Ok(v.broadcast_div(&n)?)
}

/// Текстовая башня SigLIP, где таблица слов (256 000 × hidden) не грузится в память:
/// строки для 64 токенов читаются прямо из файла. Остальное — копия слоёв candle
/// (`siglip.rs`), потому что у его `TextModel` поля закрыты.
struct Layer {
    q: Linear,
    k: Linear,
    v: Linear,
    o: Linear,
    ln1: LayerNorm,
    fc1: Linear,
    fc2: Linear,
    ln2: LayerNorm,
    heads: usize,
}

struct Lite {
    file: MmapedSafetensors,
    hidden: usize,
    pos: Tensor,
    layers: Vec<Layer>,
    final_ln: LayerNorm,
    head: Linear,
    act: candle_nn::Activation,
}

impl Lite {
    fn new(tc: &siglip::TextConfig, path: &std::path::Path, device: &Device) -> Result<Self> {
        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[path], DType::F32, device)? }.pp("text_model");
        let h = tc.hidden_size;
        let pos = vb.pp("embeddings.position_embedding").get((tc.max_position_embeddings, h), "weight")?;
        let mut layers = vec![];
        for i in 0..tc.num_hidden_layers {
            let l = vb.pp(format!("encoder.layers.{i}"));
            layers.push(Layer {
                q: linear(h, h, l.pp("self_attn.q_proj"))?,
                k: linear(h, h, l.pp("self_attn.k_proj"))?,
                v: linear(h, h, l.pp("self_attn.v_proj"))?,
                o: linear(h, h, l.pp("self_attn.out_proj"))?,
                ln1: layer_norm(h, tc.layer_norm_eps, l.pp("layer_norm1"))?,
                fc1: linear(h, tc.intermediate_size, l.pp("mlp.fc1"))?,
                fc2: linear(tc.intermediate_size, h, l.pp("mlp.fc2"))?,
                ln2: layer_norm(h, tc.layer_norm_eps, l.pp("layer_norm2"))?,
                heads: tc.num_attention_heads,
            });
        }
        Ok(Self {
            file: unsafe { MmapedSafetensors::new(path)? },
            hidden: h,
            pos,
            layers,
            final_ln: layer_norm(h, tc.layer_norm_eps, vb.pp("final_layer_norm"))?,
            head: linear(h, h, vb.pp("head"))?,
            act: tc.hidden_act,
        })
    }

    fn embed(&self, ids: &[u32]) -> Result<Tensor> {
        let view = self.file.get("text_model.embeddings.token_embedding.weight")?;
        let data = view.data();
        let width = data.len() / (data.len() / (self.hidden * 4)).max(1);
        anyhow::ensure!(width == self.hidden * 4, "таблица слов не f32: {:?}", view.dtype());
        let mut out = Vec::with_capacity(ids.len() * self.hidden);
        for &id in ids {
            let row = &data[id as usize * width..(id as usize + 1) * width];
            out.extend(row.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])));
        }
        Ok(Tensor::from_vec(out, (1, ids.len(), self.hidden), self.pos.device())?)
    }

    fn forward(&self, ids: &[u32]) -> Result<Tensor> {
        let n = ids.len();
        let mut x = self.embed(ids)?.broadcast_add(&self.pos.narrow(0, 0, n)?)?;
        for l in &self.layers {
            let r = x.clone();
            let y = x.apply(&l.ln1)?;
            let hd = self.hidden / l.heads;
            let split = |t: Tensor| -> candle_core::Result<Tensor> {
                t.reshape((1, n, l.heads, hd))?.transpose(1, 2)?.contiguous()
            };
            let (q, k, v) = (split(y.apply(&l.q)?)?, split(y.apply(&l.k)?)?, split(y.apply(&l.v)?)?);
            let w = (q.matmul(&k.t()?)? * (hd as f64).powf(-0.5))?;
            let w = candle_nn::ops::softmax_last_dim(&w)?;
            let a = w.matmul(&v)?.transpose(1, 2)?.reshape((1, n, ()))?.apply(&l.o)?;
            x = (r + a)?;
            let r = x.clone();
            let m = x.apply(&l.ln2)?.apply(&l.fc1)?.apply(&self.act)?.apply(&l.fc2)?;
            x = (m + r)?;
        }
        let last = x.apply(&self.final_ln)?.narrow(1, n - 1, 1)?.squeeze(1)?;
        Ok(last.apply(&self.head)?)
    }
}

fn main() -> Result<()> {
    let mut a = std::env::args().skip(1);
    let dir = PathBuf::from(a.next().context("arg1: каталог весов")?);
    let reference: Value =
        serde_json::from_slice(&std::fs::read(a.next().context("arg2: ref_siglip.json")?)?)?;
    let name = a.next().context("arg3: имя модели в эталоне")?;
    let mode = a.next().unwrap_or_default();
    let lite = mode == "lite";
    let dtype = if mode == "bf16" { DType::BF16 } else { DType::F32 };
    let reference = &reference[&name];
    let queries: Vec<String> = reference["queries"]
        .as_array()
        .context("queries")?
        .iter()
        .map(|q| q.as_str().unwrap_or("").to_string())
        .collect();

    let device = Device::Cpu;
    let before = rss_mb();
    let t0 = Instant::now();
    let config: siglip::Config = serde_json::from_slice(&std::fs::read(dir.join("config.json"))?)?;
    let tc = &config.text_config;
    // Только текстовая башня: тензоры зрения не читаются.
    let (model, lite_model) = if lite {
        (None, Some(Lite::new(tc, &dir.join("model.safetensors"), &device)?))
    } else {
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[dir.join("model.safetensors")], dtype, &device)?
        };
        (Some(siglip::TextModel::new(tc, vb.pp("text_model")).context("текстовая башня")?), None)
    };
    println!(
        "{name} ({mode}): загрузка {:.1} с, память {:.0} → {:.0} МБ",
        t0.elapsed().as_secs_f64(),
        before,
        rss_mb()
    );

    let tok = Tokenizer::from_file(dir.join("tokenizer.json")).map_err(|e| anyhow!("{e}"))?;
    let max_len = tc.max_position_embeddings;
    println!("pad_token_id в config = {}, длина = {max_len}", tc.pad_token_id);

    let mut ids_ok = 0;
    let mut cos_all = vec![];
    let mut times = vec![];
    for (i, q) in queries.iter().enumerate() {
        let enc = tok.encode(q.to_lowercase().as_str(), true).map_err(|e| anyhow!("{e}"))?;
        let mut ids: Vec<u32> = enc.get_ids().to_vec();
        ids.truncate(max_len);
        ids.resize(max_len, tc.pad_token_id);
        let want: Vec<u32> = reference["ids"][i]
            .as_array()
            .context("ids")?
            .iter()
            .map(|x| x.as_u64().unwrap_or(0) as u32)
            .collect();
        let same = ids == want;
        ids_ok += same as usize;

        let t = Instant::now();
        let out = match (&model, &lite_model) {
            (Some(m), _) => m.forward(&Tensor::from_vec(ids, (1, max_len), &device)?)?,
            (_, Some(l)) => l.forward(&ids)?,
            _ => unreachable!(),
        };
        let v = l2(&out.to_dtype(DType::F32)?)?.flatten_all()?.to_vec1::<f32>()?;
        times.push(t.elapsed().as_secs_f64() * 1000.0);

        let r: Vec<f32> = reference["vec"][i]
            .as_array()
            .context("vec")?
            .iter()
            .map(|x| x.as_f64().unwrap_or(0.0) as f32)
            .collect();
        let cos: f32 = v.iter().zip(&r).map(|(a, b)| a * b).sum();
        cos_all.push(cos);
        println!("  {q:32} токены {}  косинус с питоном {cos:.5}  {:.0} мс", if same { "совпали" } else { "РАЗНЫЕ" }, times[i]);
    }
    println!(
        "итог: токены {ids_ok}/{}, косинус min {:.5}, время без первого запроса {:.0} мс, память после запросов {:.0} МБ",
        queries.len(),
        cos_all.iter().cloned().fold(f32::MAX, f32::min),
        times.iter().skip(1).sum::<f64>() / (times.len().max(2) - 1) as f64,
        rss_mb()
    );
    Ok(())
}
