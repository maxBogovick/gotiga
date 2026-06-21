//! Smoke test for in-process depth generation (candle / Depth-Anything-V2).
//!
//! Verifies the model actually loads and produces a non-trivial depth map, with
//! no DB or HTTP involved. Run with the bundled weights available:
//!
//!   DEPTH_WEIGHTS_DIR=/tmp/depth-weights \
//!     cargo run --release --example depth_smoke -- <input.jpg> <out.png>
//!
//! Prints inference time and the depth map's min/max/mean so you can confirm it
//! isn't blank (a flat map = all the same value = something is wrong).
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let src = args.get(1).cloned().unwrap_or_else(|| {
        "/Users/maxim/Projects/Rust/gotiga/static/images/workshop/master-1.jpg".to_string()
    });
    let out = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| "/tmp/depth_out.png".to_string());

    eprintln!(
        "weights available: {}",
        gotiga_server::depth::is_available()
    );
    eprintln!("input : {src}");
    eprintln!("output: {out}");

    let t = std::time::Instant::now();
    gotiga_server::depth::generate(Path::new(&src), Path::new(&out))?;
    eprintln!("✓ generated in {:?}", t.elapsed());

    // Inspect the result so we can tell a real depth map from a blank one.
    let img = image::open(&out)?.to_luma8();
    let (w, h) = (img.width(), img.height());
    let (mut mn, mut mx, mut sum) = (255u8, 0u8, 0u64);
    for p in img.pixels() {
        let v = p[0];
        mn = mn.min(v);
        mx = mx.max(v);
        sum += v as u64;
    }
    let mean = sum as f64 / (w as f64 * h as f64);
    eprintln!("depth map: {w}x{h}  min={mn} max={mx} mean={mean:.1}");
    if mx == mn {
        eprintln!("⚠ depth map is FLAT (all {mn}) — inference produced nothing useful");
    } else {
        eprintln!("✓ depth map has real contrast (min≠max) — generation works");
    }
    Ok(())
}
