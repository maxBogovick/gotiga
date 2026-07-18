-- Manual markers on the analytics trend ("posted to Instagram on 14.07"),
-- entered by the admin — closes the loop between "what changed" (a post, an
-- event) and "what the graph did" without guessing from traffic alone.
CREATE TABLE IF NOT EXISTS analytics_annotations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    day DATE NOT NULL,
    label TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_analytics_annotations_day ON analytics_annotations(day);
