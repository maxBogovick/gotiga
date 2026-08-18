-- A leaf from the bench: sketches of a work still taking shape.
-- Several plates may sit on one leaf; image_url remains the cover.

ALTER TABLE gazette_leaves DROP CONSTRAINT IF EXISTS gazette_leaves_kind_check;

ALTER TABLE gazette_leaves
    ADD CONSTRAINT gazette_leaves_kind_check CHECK (kind IN (
        'arrival', 'collage', 'showing', 'guest_story', 'tale', 'note', 'world', 'sketch'
    ));

ALTER TABLE gazette_leaves
    ADD COLUMN IF NOT EXISTS image_urls TEXT[] NOT NULL DEFAULT '{}';

UPDATE gazette_leaves
   SET image_urls = ARRAY[image_url]
 WHERE image_url IS NOT NULL
   AND image_url <> ''
   AND cardinality(image_urls) = 0;
