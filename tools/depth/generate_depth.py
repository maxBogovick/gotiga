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

from depth_core import generate_one, get_estimator  # noqa: E402


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
    estimator = get_estimator(args.model)

    done = 0
    for image_id, rel_path in rows:
        status, detail = generate_one(cur, conn, estimator, upload_root, image_id, rel_path)
        if status == "done":
            done += 1
            print(f"  ✓ {image_id} → {detail}")
        elif status == "skip":
            print(f"  ! skip {image_id}: {detail}", file=sys.stderr)
        else:
            print(f"  ! fail {image_id}: {detail}", file=sys.stderr)

    cur.close()
    conn.close()
    print(f"Done. {done}/{len(rows)} depth maps written.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
