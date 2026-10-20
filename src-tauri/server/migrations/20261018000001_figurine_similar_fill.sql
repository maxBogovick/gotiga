-- Начальный выбор похожих работ для всех работ, стоящих на людях на 18.10.2026.
-- Подобран вручную: по фото каждой работы (тип героя, поза, снимок вблизи или в
-- руке) и по названию и составу. Первыми идут самые близкие.
--
-- Работы названы адресами (`slug`), а не номерами: номера в каждой базе свои,
-- адрес один и тот же. Строка, адреса которой в этой базе нет, пропускается,
-- а не роняет миграцию. `ON CONFLICT DO NOTHING` не даёт затереть выбор, уже
-- сделанный автором в админке, если миграция вдруг исполняется на живой базе.
WITH picks (slug, p0, p1, p2) AS (
  VALUES
    ('small-art-doll-forester', 'small-authors-doll-man-with-the-dog', 'tiny-ooak-wizard-art-doll-with-birds', 'tiny-ooak-gnome-art-doll-with-books-and-sparrows'),
    ('small-art-doll-granny-with-hen', 'small-doll-granny-with-chicken', 'granny-with-the-bird', 'granny-with-cats'),
    ('small-doll-witch-2', 'tiny-ooak-witch-art-doll-with-broom-and-spell-book', 'small-art-doll-witch', 'small-art-witch'),
    ('small-art-doll-frankenstein-s-creature', 'small-art-doll-quazimodo', 'small-doll-vampire', 'tiny-ooak-caveman-art-doll'),
    ('small-art-doll-black-woman-with-basket', 'small-black-woman', 'small-art-doll-model', 'small-art-doll-esmeralda'),
    ('granny-with-the-bird', 'small-doll-granny', 'small-art-doll-granny-with-hen', 'granny-with-cats'),
    ('small-art-doll-falconer', 'small-doll-indian-man', 'small-art-doll-shaman-woman', 'tiny-ooak-wizard-art-doll-with-birds'),
    ('miniature-book-fairy-tales', 'small-sculpture-book-beatrix-potter-tales', 'miniature-notre-dame-de-paris-book-sculpture', 'miniature-edgar-allan-poe-book-sculpture'),
    ('small-art-doll-shaman-woman', 'small-black-woman', 'small-art-doll-falconer', 'small-doll-indian-man'),
    ('tiny-ooak-grandma-art-doll-at-the-kitchen-table', 'small-doll-granny-with-bread', 'tiny-ooak-lady-art-doll-with-piglet', 'tiny-ooak-grandma-art-doll-with-pet-snail'),
    ('small-art-doll-tailor', 'small-art-doll-street-musician', 'small-art-doll-piper', 'ooak-porthos-musketeer-art-doll'),
    ('small-art-doll-cooking-gnome', 'small-doll-gnome-2', 'tiny-ooak-gnome-art-doll-with-books-and-sparrows', 'tiny-ooak-gnome-art-doll-with-two-mice'),
    ('granny-with-cats', 'tiny-ooak-grandma-art-doll-with-pet-snail', 'small-doll-granny', 'granny-with-the-bird'),
    ('tiny-ooak-witch-art-doll-with-broom-and-spell-book', 'tiny-ooak-red-haired-witch-art-doll-with-broom', 'small-doll-witch', 'small-doll-witch-2'),
    ('small-art-doll-lady-agatha', 'small-art-witch', 'small-art-doll-little-ghost', 'small-art-doll-witch-2'),
    ('small-authors-doll-man-with-the-dog', 'tiny-ooak-wizard-art-doll-with-birds', 'small-art-doll-forester', 'tiny-ooak-gnome-art-doll-with-books-and-sparrows'),
    ('tiny-ooak-wizard-art-doll-with-birds', 'small-authors-doll-man-with-the-dog', 'small-art-doll-forester', 'tiny-ooak-gnome-art-doll-with-books-and-sparrows'),
    ('small-art-witch', 'small-art-doll-witch', 'small-doll-witch-2', 'small-art-doll-lady-agatha'),
    ('small-baba-yaga-with-bird', 'tiny-ooak-baba-yaga-art-doll-with-broom', 'tiny-ooak-baba-yaga-art-doll-with-snakes-and-broom', 'ooak-baba-yaga-art-doll-in-mortar-with-broom'),
    ('tiny-ooak-grandma-art-doll-with-pet-snail', 'granny-with-cats', 'tiny-ooak-lady-art-doll-with-piglet', 'small-doll-granny'),
    ('small-art-doll-esmeralda', 'small-art-doll-black-woman-with-basket', 'small-black-woman', 'small-art-doll-model'),
    ('small-doll-vampire', 'small-art-doll-hamlet', 'small-art-doll-frankenstein-s-creature', 'small-art-doll-quazimodo'),
    ('tiny-ooak-gnome-art-doll-with-books-and-sparrows', 'small-art-doll-gnome-with-books', 'small-art-doll-cooking-gnome', 'small-doll-gnome-2'),
    ('small-art-doll-quazimodo', 'small-art-doll-frankenstein-s-creature', 'tiny-ooak-caveman-art-doll', 'small-doll-vampire'),
    ('small-doll-granny', 'granny-with-the-bird', 'granny-with-cats', 'small-doll-granny-with-chicken'),
    ('tiny-ooak-baba-yaga-art-doll-with-snakes-and-broom', 'tiny-ooak-baba-yaga-art-doll-with-broom', 'small-baba-yaga-with-bird', 'ooak-baba-yaga-art-doll-in-mortar-with-broom'),
    ('small-doll-witch', 'tiny-ooak-red-haired-witch-art-doll-with-broom', 'tiny-ooak-witch-art-doll-with-broom-and-spell-book', 'small-doll-witch-2'),
    ('tiny-ooak-red-haired-witch-art-doll-with-broom', 'tiny-ooak-witch-art-doll-with-broom-and-spell-book', 'small-doll-witch', 'small-art-doll-witch-2'),
    ('tiny-ooak-gnome-art-doll-with-two-mice', 'small-art-doll-gnome', 'small-art-doll-gnome-with-axe', 'small-art-doll-gnome-with-books'),
    ('small-doll-granny-with-bread', 'tiny-ooak-grandma-art-doll-at-the-kitchen-table', 'tiny-ooak-lady-art-doll-with-piglet', 'tiny-ooak-grandma-art-doll-with-pet-snail'),
    ('small-doll-gnome-2', 'small-art-doll-cooking-gnome', 'small-art-doll-gnome', 'small-art-doll-gnome-with-axe'),
    ('small-art-doll-street-musician', 'small-art-doll-piper', 'small-art-doll-pied-piper', 'small-art-doll-tailor'),
    ('small-art-doll-piper', 'small-art-doll-pied-piper', 'small-art-doll-street-musician', 'small-art-doll-tailor'),
    ('ooak-steampunk-mermaid-art-doll', 'ooak-old-mermaid-art-doll', 'ooak-old-ballerina-hanging-art-doll', 'small-art-doll-little-ghost'),
    ('small-art-doll-pied-piper', 'small-art-doll-piper', 'small-art-doll-street-musician', 'small-art-doll-red-riding-hood'),
    ('small-art-doll-hamlet', 'ooak-porthos-musketeer-art-doll', 'small-doll-vampire', 'small-art-doll-model'),
    ('tiny-ooak-caveman-art-doll', 'small-art-doll-quazimodo', 'small-art-doll-frankenstein-s-creature', 'small-doll-indian-man'),
    ('small-art-doll-model', 'small-art-doll-esmeralda', 'small-art-doll-black-woman-with-basket', 'small-black-woman'),
    ('small-art-doll-gnome-with-axe', 'small-art-doll-gnome', 'small-doll-gnome-2', 'tiny-ooak-gnome-art-doll-with-two-mice'),
    ('tiny-ooak-baba-yaga-art-doll-with-broom', 'tiny-ooak-baba-yaga-art-doll-with-snakes-and-broom', 'small-baba-yaga-with-bird', 'ooak-baba-yaga-art-doll-in-mortar-with-broom'),
    ('ooak-old-ballerina-hanging-art-doll', 'ooak-old-mermaid-art-doll', 'ooak-steampunk-mermaid-art-doll', 'small-art-doll-little-ghost'),
    ('tiny-ooak-village-girl-art-doll-with-sparrow', 'small-art-doll-red-riding-hood', 'small-doll-granny-with-chicken', 'small-doll-baba-yaga'),
    ('miniature-notre-dame-de-paris-book-sculpture', 'miniature-edgar-allan-poe-book-sculpture', 'miniature-book-fairy-tales', 'small-sculpture-book-beatrix-potter-tales'),
    ('ooak-old-mermaid-art-doll', 'ooak-steampunk-mermaid-art-doll', 'ooak-old-ballerina-hanging-art-doll', 'small-art-doll-little-ghost'),
    ('tiny-ooak-lady-art-doll-with-piglet', 'tiny-ooak-grandma-art-doll-at-the-kitchen-table', 'small-doll-granny-with-bread', 'tiny-ooak-grandma-art-doll-with-pet-snail'),
    ('miniature-edgar-allan-poe-book-sculpture', 'miniature-notre-dame-de-paris-book-sculpture', 'miniature-book-fairy-tales', 'small-sculpture-book-beatrix-potter-tales'),
    ('tiny-ooak-baba-yaga-art-doll', 'ooak-baba-yaga-art-doll-in-mortar-with-broom', 'small-doll-baba-yaga', 'small-doll-baba-yaga-2'),
    ('ooak-porthos-musketeer-art-doll', 'small-art-doll-hamlet', 'small-doll-vampire', 'small-art-doll-pied-piper'),
    ('ooak-baba-yaga-art-doll-in-mortar-with-broom', 'tiny-ooak-baba-yaga-art-doll', 'tiny-ooak-baba-yaga-art-doll-with-snakes-and-broom', 'tiny-ooak-baba-yaga-art-doll-with-broom'),
    ('small-doll-baba-yaga', 'small-doll-baba-yaga-2', 'tiny-ooak-baba-yaga-art-doll', 'small-doll-granny-with-chicken'),
    ('small-doll-gnome', 'small-art-doll-gnome', 'small-art-doll-gnome-with-axe', 'small-doll-gnome-2'),
    ('small-art-doll-little-ghost', 'ooak-old-ballerina-hanging-art-doll', 'small-art-doll-lady-agatha', 'ooak-steampunk-mermaid-art-doll'),
    ('small-art-doll-gnome', 'tiny-ooak-gnome-art-doll-with-two-mice', 'small-art-doll-gnome-with-axe', 'small-art-doll-gnome-with-books'),
    ('small-doll-indian-man', 'small-art-doll-falconer', 'tiny-ooak-caveman-art-doll', 'small-art-doll-shaman-woman'),
    ('small-black-woman', 'small-art-doll-black-woman-with-basket', 'small-art-doll-esmeralda', 'small-art-doll-shaman-woman'),
    ('small-doll-granny-with-chicken', 'small-art-doll-granny-with-hen', 'granny-with-the-bird', 'tiny-ooak-village-girl-art-doll-with-sparrow'),
    ('small-art-doll-witch', 'small-art-witch', 'small-doll-witch-2', 'tiny-ooak-witch-art-doll-with-broom-and-spell-book'),
    ('small-art-doll-red-riding-hood', 'tiny-ooak-village-girl-art-doll-with-sparrow', 'small-art-doll-pied-piper', 'small-art-doll-esmeralda'),
    ('small-sculpture-book-beatrix-potter-tales', 'miniature-book-fairy-tales', 'miniature-notre-dame-de-paris-book-sculpture', 'miniature-edgar-allan-poe-book-sculpture'),
    ('small-doll-baba-yaga-2', 'small-doll-baba-yaga', 'tiny-ooak-baba-yaga-art-doll', 'small-doll-witch'),
    ('small-art-doll-witch-2', 'tiny-ooak-red-haired-witch-art-doll-with-broom', 'small-art-doll-witch', 'tiny-ooak-witch-art-doll-with-broom-and-spell-book'),
    ('small-art-doll-monkey', 'tiny-ooak-gnome-art-doll-with-two-mice', 'tiny-ooak-caveman-art-doll', 'small-art-doll-little-ghost'),
    ('small-art-doll-gnome-with-books', 'tiny-ooak-gnome-art-doll-with-books-and-sparrows', 'small-art-doll-gnome-with-axe', 'small-art-doll-gnome')
)
INSERT INTO figurine_similar (figurine_id, similar_id, position)
SELECT f.id, t.id, k.pos
  FROM picks p
 CROSS JOIN LATERAL (VALUES (0, p.p0), (1, p.p1), (2, p.p2)) AS k (pos, target)
  JOIN figurines f ON f.slug = p.slug
  JOIN figurines t ON t.slug = k.target AND t.id <> f.id
ON CONFLICT DO NOTHING;
