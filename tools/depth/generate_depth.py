#!/usr/bin/env python3
"""
Offline monocular-depth batch for the "Living Daguerreotype" 2.5D parallax.

For every figurine image it generates a grayscale depth map with Depth-Anything-V2
and records it as `images.depth_path = 'images/depth/{image_id}.webp'`. The Svelte
`LivingDaguerreotype` component then samples that map for a few-pixel, depth-weighted
parallax. Images without a depth map fall back to in-shader luminance depth, so this
batch is purely an *enhancement* — safe to run partially, repeatedly, or never.

This lives outside the Rust hot path on purpose: depth is a one-time precompute, not
a per-request cost. Targets the server (Postgres + filesystem) deployment.

Usage:
    pip install -r requirements.txt
    DATABASE_URL=postgres://user:pass@host/db UPLOAD_DIR=/srv/gotiga/uploads \\
        python generate_depth.py [--force] [--limit N] [--model <hf-id>]

Notes:
  * CPU is fine — Depth-Anything-V2-Small is small; a handful of figurines is seconds.
  * Reads the *original* (highest-res) variant when present, else the preview.
  * Idempotent: skips images that already have depth_path unless --force.
"""

import argparse
import os
import sys
from pathlib import Path

try:
    import psycopg2
except ImportError:
    sys.exit("Missing dependency: pip install psycopg2-binary")

try:
    import numpy as np
    from PIL import Image
    from transformers import pipeline
except ImportError:
    sys.exit("Missing dependencies: pip install -r requirements.txt")


def local_source(stored: str, upload_root: "Path") -> "Path":
    """Map a stored image path to its file on disk under UPLOAD_DIR.

    The server records image paths as full public URLs
    ('https://host/static/images/original/x.jpg') or '/static/...' or already
    relative ('images/original/x.jpg'). On disk the API serves '/static/images'
    from '<UPLOAD_DIR>/images', so we keep everything after '/static/'."""
    p = (stored or "").split("?", 1)[0].split("#", 1)[0]  # drop any query/fragment
    marker = "/static/"
    idx = p.find(marker)
    if idx != -1:
        p = p[idx + len(marker):]
    elif "://" in p:                      # URL without /static/ — drop scheme+host
        p = p.split("/", 3)[-1]
    return upload_root / p.lstrip("/")


def normalize_depth(depth_img: "Image.Image") -> "Image.Image":
    """Stretch the predicted depth to full 0..255 so the shader's [0,1] range is used.

    Convention: brighter = nearer. Depth-Anything outputs larger = nearer already,
    which matches the component (foreground shifts most with the pointer)."""
    arr = np.asarray(depth_img, dtype=np.float32)
    lo, hi = float(arr.min()), float(arr.max())
    if hi - lo < 1e-6:
        arr[:] = 128.0
    else:
        arr = (arr - lo) / (hi - lo) * 255.0
    return Image.fromarray(arr.astype(np.uint8), mode="L")


def main() -> int:
    ap = argparse.ArgumentParser(description="Generate monocular depth maps for figurine images.")
    ap.add_argument("--force", action="store_true", help="Regenerate even if depth_path is set.")
    ap.add_argument("--limit", type=int, default=0, help="Process at most N images (0 = all).")
    ap.add_argument("--model", default="depth-anything/Depth-Anything-V2-Small-hf",
                    help="HF depth-estimation model id.")
    args = ap.parse_args()

    db_url = os.environ.get("DATABASE_URL")
    upload_dir = os.environ.get("UPLOAD_DIR")
    if not db_url or not upload_dir:
        return print("Set DATABASE_URL and UPLOAD_DIR.", file=sys.stderr) or 2

    upload_root = Path(upload_dir)
    depth_dir = upload_root / "images" / "depth"
    depth_dir.mkdir(parents=True, exist_ok=True)

    conn = psycopg2.connect(db_url)
    conn.autocommit = False
    cur = conn.cursor()

    where = "" if args.force else "WHERE depth_path IS NULL"
    cur.execute(
        f"SELECT id::text, COALESCE(original_path, file_path) FROM images {where} ORDER BY id"
    )
    rows = cur.fetchall()
    if args.limit > 0:
        rows = rows[: args.limit]
    if not rows:
        print("Nothing to do — all images already have depth maps.")
        return 0

    print(f"Loading model {args.model} …")
    estimator = pipeline(task="depth-estimation", model=args.model)

    done = 0
    for image_id, rel_path in rows:
        src = local_source(rel_path, upload_root)
        if not src.exists():
            print(f"  ! skip {image_id}: source missing ({rel_path} → {src})", file=sys.stderr)
            continue
        try:
            img = Image.open(src).convert("RGB")
            result = estimator(img)
            depth = normalize_depth(result["depth"])
            # Match the colour image's pixel grid so the shader's UVs line up exactly.
            if depth.size != img.size:
                depth = depth.resize(img.size, Image.BILINEAR)

            rel_depth = f"images/depth/{image_id}.webp"
            depth.save(upload_root / rel_depth, format="WEBP", quality=80, method=6)
            cur.execute("UPDATE images SET depth_path = %s WHERE id = %s", (rel_depth, image_id))
            conn.commit()
            done += 1
            print(f"  ✓ {image_id} → {rel_depth}")
        except Exception as exc:  # noqa: BLE001 — keep the batch going
            conn.rollback()
            print(f"  ! fail {image_id}: {exc}", file=sys.stderr)

    cur.close()
    conn.close()
    print(f"Done. {done}/{len(rows)} depth maps written.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
