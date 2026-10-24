-- Обложки книг небылиц и надписи на них (название и имя автора).
-- Картинки лежат в assets/covers/ и кладутся на склад при запуске.
-- Обложка ставится только там, где её ещё нет: загруженная вручную остаётся.
-- Пустой текст надписи: название — байки на языке читателя, автор — из листа.

UPDATE gazette_leaves
   SET book_cover = '/static/covers/book-molodost-bystree-tvoih.webp',
       book_cover_text = '{"title":{"shown":true,"text":"","x":50,"y":78.2,"width":84.0,"font":"cormorant","size":9.0,"weight":700,"lineHeight":1.08,"spacing":0.01,"italic":false,"shadow":1,"align":"center","color":"#f6ead0","plate":{"on":false,"color":"#f3ead9","opacity":0.55,"blur":1.5,"radius":1.5,"pad":3.0}},"author":{"shown":true,"text":"","x":50,"y":94.5,"width":60,"font":"cormorant","size":5.6,"weight":600,"lineHeight":1.15,"spacing":0.1,"italic":true,"shadow":1,"align":"center","color":"#ecd9a8","plate":{"on":false,"color":"#f3ead9","opacity":0.55,"blur":1.5,"radius":1.5,"pad":3.0}}}'::jsonb
 WHERE slug = 'molodost-bystree-tvoih' AND kind = 'tale' AND book_cover IS NULL;

UPDATE gazette_leaves
   SET book_cover = '/static/covers/book-ubit-a-potom-razorvat.webp',
       book_cover_text = '{"title":{"shown":true,"text":"","x":50,"y":78.2,"width":84.0,"font":"cormorant","size":9.0,"weight":700,"lineHeight":1.08,"spacing":0.01,"italic":false,"shadow":1,"align":"center","color":"#f6ead0","plate":{"on":false,"color":"#f3ead9","opacity":0.55,"blur":1.5,"radius":1.5,"pad":3.0}},"author":{"shown":true,"text":"","x":50,"y":94.5,"width":60,"font":"cormorant","size":5.6,"weight":600,"lineHeight":1.15,"spacing":0.1,"italic":true,"shadow":1,"align":"center","color":"#ecd9a8","plate":{"on":false,"color":"#f3ead9","opacity":0.55,"blur":1.5,"radius":1.5,"pad":3.0}}}'::jsonb
 WHERE slug = 'ubit-a-potom-razorvat' AND kind = 'tale' AND book_cover IS NULL;

UPDATE gazette_leaves
   SET book_cover = '/static/covers/book-vse-kak-u-nas.webp',
       book_cover_text = '{"title":{"shown":true,"text":"","x":50,"y":77.5,"width":84.0,"font":"cormorant","size":10.0,"weight":700,"lineHeight":1.08,"spacing":0.01,"italic":false,"shadow":1,"align":"center","color":"#f6ead0","plate":{"on":false,"color":"#f3ead9","opacity":0.55,"blur":1.5,"radius":1.5,"pad":3.0}},"author":{"shown":true,"text":"","x":50,"y":94.5,"width":60,"font":"cormorant","size":5.6,"weight":600,"lineHeight":1.15,"spacing":0.1,"italic":true,"shadow":1,"align":"center","color":"#ecd9a8","plate":{"on":false,"color":"#f3ead9","opacity":0.55,"blur":1.5,"radius":1.5,"pad":3.0}}}'::jsonb
 WHERE slug = 'vse-kak-u-nas' AND kind = 'tale' AND book_cover IS NULL;

UPDATE gazette_leaves
   SET book_cover = '/static/covers/book-the-forester-and-his-unforgettable-memory.webp',
       book_cover_text = '{"title":{"shown":true,"text":"","x":50,"y":76.1,"width":84.0,"font":"cormorant","size":8.0,"weight":700,"lineHeight":1.08,"spacing":0.01,"italic":false,"shadow":1,"align":"center","color":"#f6ead0","plate":{"on":false,"color":"#f3ead9","opacity":0.55,"blur":1.5,"radius":1.5,"pad":3.0}},"author":{"shown":true,"text":"","x":50,"y":94.5,"width":60,"font":"cormorant","size":5.6,"weight":600,"lineHeight":1.15,"spacing":0.1,"italic":true,"shadow":1,"align":"center","color":"#ecd9a8","plate":{"on":false,"color":"#f3ead9","opacity":0.55,"blur":1.5,"radius":1.5,"pad":3.0}}}'::jsonb
 WHERE slug = 'the-forester-and-his-unforgettable-memory' AND kind = 'tale' AND book_cover IS NULL;

UPDATE gazette_leaves
   SET book_cover = '/static/covers/book-gb.webp',
       book_cover_text = '{"title":{"shown":true,"text":"","x":50,"y":77.5,"width":84.0,"font":"cormorant","size":10.0,"weight":700,"lineHeight":1.08,"spacing":0.01,"italic":false,"shadow":1,"align":"center","color":"#f6ead0","plate":{"on":false,"color":"#f3ead9","opacity":0.55,"blur":1.5,"radius":1.5,"pad":3.0}},"author":{"shown":true,"text":"","x":50,"y":94.5,"width":60,"font":"cormorant","size":5.6,"weight":600,"lineHeight":1.15,"spacing":0.1,"italic":true,"shadow":1,"align":"center","color":"#ecd9a8","plate":{"on":false,"color":"#f3ead9","opacity":0.55,"blur":1.5,"radius":1.5,"pad":3.0}}}'::jsonb
 WHERE slug = 'gb' AND kind = 'tale' AND book_cover IS NULL;

UPDATE gazette_leaves
   SET book_cover = '/static/covers/book-mir-ne-takoy-kak-tebe.webp',
       book_cover_text = '{"title":{"shown":true,"text":"","x":50,"y":77.5,"width":84.0,"font":"cormorant","size":10.0,"weight":700,"lineHeight":1.08,"spacing":0.01,"italic":false,"shadow":1,"align":"center","color":"#f6ead0","plate":{"on":false,"color":"#f3ead9","opacity":0.55,"blur":1.5,"radius":1.5,"pad":3.0}},"author":{"shown":true,"text":"","x":50,"y":94.5,"width":60,"font":"cormorant","size":5.6,"weight":600,"lineHeight":1.15,"spacing":0.1,"italic":true,"shadow":1,"align":"center","color":"#ecd9a8","plate":{"on":false,"color":"#f3ead9","opacity":0.55,"blur":1.5,"radius":1.5,"pad":3.0}}}'::jsonb
 WHERE slug = 'mir-ne-takoy-kak-tebe' AND kind = 'tale' AND book_cover IS NULL;

UPDATE gazette_leaves
   SET book_cover = '/static/covers/book-otmeryalschik-ot-boga.webp',
       book_cover_text = '{"title":{"shown":true,"text":"","x":50,"y":77.5,"width":84.0,"font":"cormorant","size":10.0,"weight":700,"lineHeight":1.08,"spacing":0.01,"italic":false,"shadow":1,"align":"center","color":"#f6ead0","plate":{"on":false,"color":"#f3ead9","opacity":0.55,"blur":1.5,"radius":1.5,"pad":3.0}},"author":{"shown":true,"text":"","x":50,"y":94.5,"width":60,"font":"cormorant","size":5.6,"weight":600,"lineHeight":1.15,"spacing":0.1,"italic":true,"shadow":1,"align":"center","color":"#ecd9a8","plate":{"on":false,"color":"#f3ead9","opacity":0.55,"blur":1.5,"radius":1.5,"pad":3.0}}}'::jsonb
 WHERE slug = 'otmeryalschik-ot-boga' AND kind = 'tale' AND book_cover IS NULL;

UPDATE gazette_leaves
   SET book_cover = '/static/covers/book-ispolnyu-vse-zhelaniya.webp',
       book_cover_text = '{"title":{"shown":true,"text":"","x":50,"y":77.5,"width":84.0,"font":"cormorant","size":10.0,"weight":700,"lineHeight":1.08,"spacing":0.01,"italic":false,"shadow":1,"align":"center","color":"#f6ead0","plate":{"on":false,"color":"#f3ead9","opacity":0.55,"blur":1.5,"radius":1.5,"pad":3.0}},"author":{"shown":true,"text":"","x":50,"y":94.5,"width":60,"font":"cormorant","size":5.6,"weight":600,"lineHeight":1.15,"spacing":0.1,"italic":true,"shadow":1,"align":"center","color":"#ecd9a8","plate":{"on":false,"color":"#f3ead9","opacity":0.55,"blur":1.5,"radius":1.5,"pad":3.0}}}'::jsonb
 WHERE slug = 'ispolnyu-vse-zhelaniya' AND kind = 'tale' AND book_cover IS NULL;

UPDATE gazette_leaves
   SET book_cover = '/static/covers/book-ya-veryu.webp',
       book_cover_text = '{"title":{"shown":true,"text":"","x":50,"y":77.5,"width":84.0,"font":"cormorant","size":10.0,"weight":700,"lineHeight":1.08,"spacing":0.01,"italic":false,"shadow":1,"align":"center","color":"#f6ead0","plate":{"on":false,"color":"#f3ead9","opacity":0.55,"blur":1.5,"radius":1.5,"pad":3.0}},"author":{"shown":true,"text":"","x":50,"y":94.5,"width":60,"font":"cormorant","size":5.6,"weight":600,"lineHeight":1.15,"spacing":0.1,"italic":true,"shadow":1,"align":"center","color":"#ecd9a8","plate":{"on":false,"color":"#f3ead9","opacity":0.55,"blur":1.5,"radius":1.5,"pad":3.0}}}'::jsonb
 WHERE slug = 'ya-veryu' AND kind = 'tale' AND book_cover IS NULL;

UPDATE gazette_leaves
   SET book_cover = '/static/covers/book-ne-odinoko-grustno.webp',
       book_cover_text = '{"title":{"shown":true,"text":"","x":50,"y":77.5,"width":84.0,"font":"cormorant","size":10.0,"weight":700,"lineHeight":1.08,"spacing":0.01,"italic":false,"shadow":1,"align":"center","color":"#f6ead0","plate":{"on":false,"color":"#f3ead9","opacity":0.55,"blur":1.5,"radius":1.5,"pad":3.0}},"author":{"shown":true,"text":"","x":50,"y":94.5,"width":60,"font":"cormorant","size":5.6,"weight":600,"lineHeight":1.15,"spacing":0.1,"italic":true,"shadow":1,"align":"center","color":"#ecd9a8","plate":{"on":false,"color":"#f3ead9","opacity":0.55,"blur":1.5,"radius":1.5,"pad":3.0}}}'::jsonb
 WHERE slug = 'ne-odinoko-grustno' AND kind = 'tale' AND book_cover IS NULL;

UPDATE gazette_leaves
   SET book_cover = '/static/covers/book-lechu.webp',
       book_cover_text = '{"title":{"shown":true,"text":"","x":50,"y":76.1,"width":84.0,"font":"cormorant","size":8.0,"weight":700,"lineHeight":1.08,"spacing":0.01,"italic":false,"shadow":1,"align":"center","color":"#f6ead0","plate":{"on":false,"color":"#f3ead9","opacity":0.55,"blur":1.5,"radius":1.5,"pad":3.0}},"author":{"shown":true,"text":"","x":50,"y":94.5,"width":60,"font":"cormorant","size":5.6,"weight":600,"lineHeight":1.15,"spacing":0.1,"italic":true,"shadow":1,"align":"center","color":"#ecd9a8","plate":{"on":false,"color":"#f3ead9","opacity":0.55,"blur":1.5,"radius":1.5,"pad":3.0}}}'::jsonb
 WHERE slug = 'lechu' AND kind = 'tale' AND book_cover IS NULL;
