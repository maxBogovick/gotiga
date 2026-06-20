# Depth maps — "Living Daguerreotype"

Precomputes monocular depth maps so figurine photos gain a subtle, depth-weighted
2.5D parallax on the detail page (`src/lib/components/LivingDaguerreotype.svelte`).

## How the pieces fit

```
generate_depth.py ──> {UPLOAD_DIR}/images/depth/{image_id}.webp   (grayscale)
                 └──> UPDATE images SET depth_path = 'images/depth/{image_id}.webp'

API (Rust) ──> ImageDto.depthUrl  ──>  resolveMediaUrl()  ──>  <LivingDaguerreotype depthSrc=…>
```

No depth map? The component derives depth from the image's luminance in-shader, so the
effect already works archive-wide — running this batch just upgrades the fidelity
(true subject/ground separation instead of a brightness heuristic).

## Run

```bash
cd tools/depth
python -m venv .venv && source .venv/bin/activate
pip install -r requirements.txt

DATABASE_URL=postgres://user:pass@host/gotiga \
UPLOAD_DIR=/srv/gotiga/uploads \
python generate_depth.py            # only images lacking a depth map
# --force   regenerate all
# --limit N first N images (smoke test)
# --model   swap the HF model (default Depth-Anything-V2-Small-hf)
```

CPU is sufficient (small model; seconds per image). The batch is idempotent and commits
per image, so it is safe to interrupt and re-run.

## Run on the server (Docker, no Python setup)

The deploy stack ships a profile-gated `depth` service that wraps this script in an
image with torch + the model baked in. It reaches the live Postgres + uploads volume
over the compose network, so there's nothing to install on the box.

```bash
# Once (on your build machine): build & push the depth image — only when this
# script or its deps change, NOT on every app deploy.
REGISTRY=docker.io/<you> deploy/depth/build-and-push-depth.sh

# On the server: generate any missing depth maps against the running stack.
/opt/gotiga/run-depth.sh                 # only-missing (idempotent)
/opt/gotiga/run-depth.sh --force         # regenerate all
/opt/gotiga/run-depth.sh --limit 5       # smoke test
```

To fold it into the deploy itself, set `RUN_DEPTH_ON_DEPLOY=1` in `/opt/gotiga/.env` —
`remote-deploy.sh` then runs the batch (best-effort, non-fatal) after the API is healthy.

The raw script below is the local / non-Docker path (e.g. against a tunnelled DB).

## Prerequisites

* Migration `20260620000002_image_depth_map.sql` applied (adds `images.depth_path`).
* `UPLOAD_DIR` is the same directory the API serves under `/static/` and writes uploads to.

## Scope

Targets the **server (Postgres + filesystem)** deployment — the public web build. The Tauri
desktop app stores images as SQLite BLOBs and is the admin tool; depth there falls back to
the luminance path. The `depth_path` column and DTO field exist on both backends, so the
desktop app round-trips depth maps without error if ever populated.
