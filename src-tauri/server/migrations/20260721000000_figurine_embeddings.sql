-- Semantic search ("Хранитель"): one dense text embedding per figurine.
--
-- Each figurine's curatorial text (name + short/full description + material +
-- technique …) is encoded into a single L2-normalised vector by an on-device
-- multilingual bi-encoder (candle, see server/src/embed.rs). Natural-language
-- guest queries are encoded the same way and ranked by cosine similarity.
--
-- The corpus is tiny (tens of works), so there is deliberately NO vector index
-- here: a brute-force dot product over a few dozen rows in memory is instant and
-- far simpler than pgvector/ANN. `vec` is a raw little-endian f32[dim] BYTEA.
CREATE TABLE IF NOT EXISTS figurine_embeddings (
    figurine_id UUID PRIMARY KEY REFERENCES figurines(id) ON DELETE CASCADE,
    -- Which model produced this vector, e.g. 'intfloat/multilingual-e5-small'.
    -- Search only compares vectors sharing the *current* model, so swapping the
    -- model simply makes old rows dormant until they are re-indexed.
    model TEXT NOT NULL,
    dim INTEGER NOT NULL,
    -- L2-normalised embedding as little-endian f32; length = dim * 4 bytes.
    vec BYTEA NOT NULL,
    -- sha256(model || '\n' || embedded_text): lets re-index skip a figurine whose
    -- text (and model) are unchanged, so a full reindex is cheap after the first.
    source_hash TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
