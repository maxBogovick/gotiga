"""Shared depth-map generation core, used by both the CLI batch
(generate_depth.py) and the on-demand HTTP service (serve.py)."""

import sys
from pathlib import Path

try:
    import numpy as np
    from PIL import Image
    from transformers import pipeline
except ImportError:
    sys.exit("Missing dependencies: pip install -r requirements.txt")


def local_source(stored: str, upload_root: Path) -> Path:
    """Map a stored image path to its file on disk under UPLOAD_DIR.

    The server records image paths as full public URLs
    ('https://host/static/images/original/x.jpg'), '/static/...' or already
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


def get_estimator(model_id: str):
    """Build a depth-estimation pipeline. CPU is fine for the small model."""
    return pipeline(task="depth-estimation", model=model_id)


def generate_one(cur, conn, estimator, upload_root: Path, image_id: str, stored_path: str):
    """Generate + persist one depth map. Commits per image (safe to interrupt).

    Returns (status, detail) where status is 'done' | 'skip' | 'fail'."""
    src = local_source(stored_path, upload_root)
    if not src.exists():
        return ("skip", f"source missing ({stored_path} → {src})")
    try:
        img = Image.open(src).convert("RGB")
        result = estimator(img)
        depth = normalize_depth(result["depth"])
        # Match the colour image's pixel grid so the shader's UVs line up exactly.
        if depth.size != img.size:
            depth = depth.resize(img.size, Image.BILINEAR)

        (upload_root / "images" / "depth").mkdir(parents=True, exist_ok=True)
        rel_depth = f"images/depth/{image_id}.webp"
        depth.save(upload_root / rel_depth, format="WEBP", quality=80, method=6)
        cur.execute("UPDATE images SET depth_path = %s WHERE id = %s", (rel_depth, image_id))
        conn.commit()
        return ("done", rel_depth)
    except Exception as exc:  # noqa: BLE001 — keep the batch/service alive
        conn.rollback()
        return ("fail", str(exc))
