-- Похожие работы для работ, добавленных после первого выбора
-- (`20261018000001_figurine_similar_fill.sql`). Подобраны по фото: тип героя,
-- поза и снимок, затем название и состав. Первыми идут самые близкие.
--
-- Работы названы адресами (`slug`): номера в каждой базе свои. Незнакомый адрес
-- пропускается, `ON CONFLICT DO NOTHING` не затирает выбор, уже сделанный
-- автором в админке.
WITH picks (slug, p0, p1, p2) AS (
  VALUES
    ('granny-baba-yaga', 'tiny-ooak-baba-yaga-art-doll-with-broom', 'small-doll-sweet-baba-yaga', 'tiny-ooak-baba-yaga-art-doll-with-snakes-and-broom'),
    ('small-doll-gnome-woman', 'little-guy-with-pumpkins', 'old-lady-gnome', 'small-doll-gnome-with-the-book'),
    ('tiny-with-with-broom', 'small-doll-witch', 'tiny-ooak-red-haired-witch-art-doll-with-broom', 'tiny-ooak-witch-art-doll-with-broom-and-spell-book'),
    ('small-authors-witch', 'small-art-doll-witch', 'small-doll-witch-in-black', 'small-art-witch'),
    ('little-guy-with-pumpkins', 'small-doll-gnome-with-the-book', 'small-doll-gnome-woman', 'small-art-doll-gnome-2'),
    ('small-doll-gnome-with-the-book', 'small-doll-gnome-woman', 'little-guy-with-pumpkins', 'small-art-doll-gnome-2'),
    ('quaismodo-doll', 'small-art-doll-quazimodo', 'miniature-notre-dame-de-paris-book-sculpture', 'small-art-doll-frankenstein-s-creature'),
    ('small-art-doll-wizard', 'tiny-ooak-wizard-art-doll-with-birds', 'tiny-ooak-gnome-art-doll-with-books-and-sparrows', 'small-authors-doll-man-with-the-dog'),
    ('old-lady-gnome', 'small-doll-gnome-woman', 'small-doll-gnome-with-fancy-bag', 'little-guy-with-pumpkins'),
    ('green-gnome-gardener', 'small-art-doll-cooking-gnome', 'small-art-doll-gnome-with-gold', 'small-art-doll-gnome-with-books'),
    ('small-doll-sweet-baba-yaga', 'tiny-ooak-baba-yaga-art-doll-with-broom', 'granny-baba-yaga', 'small-doll-baba-yaga'),
    ('small-art-doll-shaman', 'small-art-doll-shaman-woman', 'small-art-doll-falconer', 'small-doll-indian-man'),
    ('small-doll-with-with-nest', 'small-baba-yaga-with-bird', 'small-doll-witch', 'small-doll-baba-yaga'),
    ('small-doll-gnome-with-fancy-bag', 'small-doll-gnome-with-bag', 'small-doll-pirate-with-parrot', 'tiny-ooak-gnome-art-doll-with-two-mice'),
    ('small-doll-lady-with-pigeons', 'tiny-ooak-village-girl-art-doll-with-sparrow', 'small-art-doll-lady-with-cat', 'granny-with-the-bird'),
    ('tiny-vampire', 'small-authors-doll-tiny-vapmire', 'small-doll-little-vampire', 'small-doll-vampire'),
    ('small-doll-granny-with-her-sheep', 'small-art-doll-granny-with-hen', 'small-doll-granny', 'granny-with-cats'),
    ('small-doll-witch-in-black', 'small-art-doll-witch', 'small-art-witch', 'small-doll-witch-2'),
    ('art-doll-captainsalazar', 'jared-leto-doll', 'small-art-doll-hamlet', 'ooak-porthos-musketeer-art-doll'),
    ('small-art-doll-gnome-2', 'small-doll-gnome-with-the-book', 'little-guy-with-pumpkins', 'small-doll-gnome-woman'),
    ('small-art-doll-gnome-with-gold', 'green-gnome-gardener', 'small-art-doll-cooking-gnome', 'small-art-doll-gnome-with-books'),
    ('witch-with-scull', 'small-art-doll-witch-2', 'small-doll-witch-in-black', 'small-art-doll-witch'),
    ('small-doll-pirate-with-parrot', 'tiny-ooak-caveman-art-doll', 'small-doll-gnome-with-fancy-bag', 'small-art-doll-gnome-with-axe'),
    ('small-authors-doll-tiny-vapmire', 'tiny-vampire', 'small-doll-vampire', 'small-doll-little-vampire'),
    ('small-art-doll-lady-with-cat', 'small-doll-lady-with-pigeons', 'tiny-with-with-broom', 'tiny-ooak-red-haired-witch-art-doll-with-broom'),
    ('small-doll-gnome-with-bag', 'small-doll-gnome-with-fancy-bag', 'small-doll-gnome-with-the-book', 'small-doll-pirate-with-parrot'),
    ('jared-leto-doll', 'art-doll-captainsalazar', 'small-art-doll-hamlet', 'ooak-porthos-musketeer-art-doll'),
    ('small-doll-little-vampire', 'tiny-vampire', 'small-authors-doll-tiny-vapmire', 'small-doll-vampire')
)
INSERT INTO figurine_similar (figurine_id, similar_id, position)
SELECT f.id, t.id, k.pos
  FROM picks p
 CROSS JOIN LATERAL (VALUES (0, p.p0), (1, p.p1), (2, p.p2)) AS k (pos, target)
  JOIN figurines f ON f.slug = p.slug
  JOIN figurines t ON t.slug = k.target AND t.id <> f.id
ON CONFLICT DO NOTHING;
