# Professional Server Backend Specification for Gotiga

## 1. Executive Summary
This document specifies the architecture and API for the centralized Gotiga Backend Server. This server acts as the authoritative source of truth for the Gotiga ecosystem, replacing the current local-only storage and static file synchronization. It is designed to be consumed by the existing Desktop (Tauri) application, future web clients, and admin tools.

**Key Objectives:**
1.  **Centralized Persistence:** Migrate from decentralized SQLite/JSON to a robust PostgreSQL database.
2.  **Backward Compatible Sync:** Provide endpoints that mimic the existing `manifest.json` structure to ease migration for the Desktop Client.
3.  **Secure Asset Management:** Centralize image/video upload and hosting.
4.  **Scalable Architecture:** Use a layered architecture (Controller-Service-Repository) in Rust.

## 2. Technology Stack

*   **Runtime:** Rust (Stable)
*   **Web Framework:** **Axum** (Chosen for ergonomics, performance, and strong ecosystem).
*   **Database:** **PostgreSQL 15+** (Production standard).
*   **ORM / Data Access:** **SQLx** (Async, compile-time checked SQL).
*   **Serialization:** Serde / Serde JSON.
*   **Storage:** File System (MVP) with interface abstraction for S3-compatible storage (MinIO/AWS/R2).
*   **Containerization:** Docker & Docker Compose.

## 3. System Architecture

The application shall follow a **Clean Layered Architecture** to separate concerns:

1.  **Transport Layer (API):** Axum Handlers. Responsible for HTTP parsing, validation, and serialization.
2.  **Service Layer (Business Logic):** Handles complex logic (e.g., "Related Items" calculation, "Manifest Generation").
3.  **Repository Layer (Persistence):** SQLx queries. No business logic here, only CRUD.
4.  **Storage Layer (Assets):** Abstract trait `FileStorage` to handle saving/retrieving blobs.

### Middleware
*   **CORS:** Permissive for development, restricted to trusted domains in production.
*   **Logging/Tracing:** `tracing` crate with `tracing-subscriber` (JSON logs in prod).
*   **Authentication:** Bearer Token middleware for write/admin operations.
*   **Static Files:** Middleware to serve uploaded assets from the local volume.

## 4. Database Schema (PostgreSQL)

The schema mirrors the SQLite structure but utilizes PostgreSQL features (UUIDs, Timestamps).

```sql
-- Enums
CREATE TYPE figurine_status AS ENUM ('available', 'sold', 'reserved');
CREATE TYPE image_type AS ENUM ('face', 'detail', 'full');
CREATE TYPE step_type AS ENUM ('sketch', 'prototype', 'modeling', 'painting', 'finish');
CREATE TYPE zone_type AS ENUM ('showcase', 'desk', 'shelf', 'note');
CREATE TYPE text_category AS ENUM ('author', 'workshop');

-- Tables
CREATE TABLE figurines (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    short_text TEXT,
    full_description TEXT,
    dimensions TEXT,
    material TEXT,
    technique TEXT,
    year INTEGER,
    ambience_path TEXT, -- Stored as relative path "ambience/uuid.mp3"
    video_url TEXT,     -- Stored as relative path "videos/uuid.mp4"
    secret_text TEXT,
    is_visible BOOLEAN NOT NULL DEFAULT true,
    status figurine_status NOT NULL DEFAULT 'available',
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE images (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    figurine_id UUID NOT NULL REFERENCES figurines(id) ON DELETE CASCADE,
    image_type image_type NOT NULL,
    file_path TEXT NOT NULL, -- "images/uuid.jpg"
    alt_text TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE process_steps (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    figurine_id UUID NOT NULL REFERENCES figurines(id) ON DELETE CASCADE,
    step_type step_type NOT NULL,
    description TEXT,
    image_path TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE texts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    category text_category NOT NULL,
    content TEXT NOT NULL,
    caption TEXT,
    image_path TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE cabinet_zones (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    zone_type zone_type NOT NULL,
    x_percent DOUBLE PRECISION NOT NULL,
    y_percent DOUBLE PRECISION NOT NULL,
    width_percent DOUBLE PRECISION NOT NULL,
    height_percent DOUBLE PRECISION NOT NULL,
    target_route TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0
);
```

## 5. API Specification

**Base URL:** `/api/v1`

### 5.1 Public Endpoints (Read-Only)

#### `GET /health`
*   **Purpose:** Health check for load balancers.
*   **Response:** `200 OK` `{"status": "ok", "version": "1.0.0"}`

#### `GET /sync/manifest`
*   **Purpose:** Backward compatibility for Desktop App synchronization.
*   **Logic:** Generates a JSON object matching the `Manifest` struct in `sync_service.rs`.
*   **Response:**
    ```json
    {
      "version": 1705492200, // Unix Timestamp
      "generatedAt": "2024-01-17T...",
      "figurines": [ ...db_rows... ],
      "images": [ ...db_rows... ],
      "processSteps": [ ...db_rows... ]
    }
    ```
    *Note: Paths in this response must be relative (e.g., "images/abc.jpg"). The client resolves these against the static file base URL.*

#### `GET /figurines`
*   **Query Params:** `?visible=true` (default), `?sort=order`
*   **Response:** `Vec<FigurineListItemDto>`

#### `GET /figurines/:id`
*   **Response:** `FigurineDto` (Aggregated with Images, Steps, and "Related Items").
*   **Related Items Logic:** Same as current client (Year match OR Material match, Random 3).

#### `GET /content/texts/:category`
*   **Params:** `category` ("author" | "workshop")
*   **Response:** `Vec<TextDto>` or `Vec<WorkshopItemDto>`

#### `GET /cabinet/zones`
*   **Purpose:** Returns interactive zones for the virtual cabinet view.
*   **Response:** `Vec<CabinetZoneDto>`

### 5.2 Admin Endpoints (Authenticated)
**Header:** `Authorization: Bearer <API_KEY>`

#### `POST /figurines` (Upsert)
*   **Body:** `FigurineDto` (Full object structure).
*   **Behavior:** Atomic replacement.
    1.  Upsert Figurine fields.
    2.  Delete *all* existing images/steps for this ID.
    3.  Insert new images/steps from the payload.
    *Rationale: Simplifies client logic; exact match of current client's "Save" behavior.*

#### `DELETE /figurines/:id`
*   **Purpose:** Permanently remove a figurine.
*   **Behavior:** Deletes the figurine row. Cascading delete removes associated images and process steps.
*   **Response:** `204 No Content`

#### `POST /upload`
*   **Content-Type:** `multipart/form-data`
*   **Field:** `file`, `type` ("image" | "video")
*   **Behavior:**
    1.  Generate UUID filename.
    2.  Save to disk: `./uploads/{type}s/{uuid}.ext`.
    3.  Return public URL.
*   **Response:**
    ```json
    {
      "url": "https://api.gotiga.com/static/images/550e8400-e29b-41d4-a716-446655440000.jpg",
      "relativePath": "images/550e8400-e29b-41d4-a716-446655440000.jpg"
    }
    ```

### 5.3 Static Asset Serving
*   **URL:** `/static/*`
*   **Implementation:** `axum::routing::get_service(ServeDir::new("./uploads"))`
*   **Maps to:** Local `./uploads` directory.

## 6. Implementation Guidelines

### 6.1 Configuration (Env Vars)
*   `DATABASE_URL`: Postgres connection string.
*   `HOST`: `0.0.0.0`
*   `PORT`: `3000`
*   `ADMIN_API_KEY`: Secret key for admin routes.
*   `UPLOAD_DIR`: Path to store files (default: `./uploads`).
*   `PUBLIC_URL`: The domain name (e.g., `https://api.gotiga.com`) used for generating full URLs.

### 6.2 Error Handling
Use a standardized error enum that implements `IntoResponse`.

```rust
#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    details: Option<String>,
}
```
*   `404 Not Found`: Entity missing.
*   `400 Bad Request`: Validation failure.
*   `401 Unauthorized`: Missing/Invalid API Key.
*   `500 Internal Server Error`: DB or IO failure.

### 6.3 Docker Deployment
Provide a `Dockerfile` and `docker-compose.yml`.
*   **App Service:** Builds the Rust binary. Mounts `./uploads` volume.
*   **DB Service:** Postgres 15. Mounts `pgdata` volume.

## 7. Migration Plan (Desktop Client)
1.  **Step 1:** Modify `SyncService` in Desktop App.
    *   Change `MANIFEST_URL` to `<SERVER_URL>/api/v1/sync/manifest`.
    *   Change `CLOUD_BASE_URL` to `<SERVER_URL>/static`.
    *   This allows the desktop app to "pull" from the server as if it were the R2 bucket.
2.  **Step 2:** Update Admin Panel in Desktop App.
    *   Direct `save_figurine` calls to `POST <SERVER_URL>/api/v1/figurines`.
    *   Direct file imports to `POST <SERVER_URL>/api/v1/upload`.

## 8. Security Considerations
*   **Input Validation:** Sanitize all text fields.
*   **File Uploads:** Validate mime-types (magic bytes) to prevent executable uploads. Limit file size (e.g., 100MB for video).
*   **Path Traversal:** Ensure requested static files cannot escape the upload directory.
