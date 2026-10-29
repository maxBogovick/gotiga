-- Переводы шести работ, у которых не было ни одного перевода (прод, 26.10.2026):
-- granny-baba-yaga, green-gnome-gardener, little-guy-with-pumpkins,
-- old-lady-gnome, quaismodo-doll, tiny-with-with-broom.
-- Языки: ru, uk, es, de, it. Поля: name, short_text, full_description, material, technique
-- (остальные поля у этих работ пусты и на английском).
--
-- Запускать вручную на сервере через psql к базе сайта.
--
-- Можно запускать повторно: работа ищется по slug (нет такого slug — ничего не вставится),
-- у существующей строки перевода заполняются только пустые колонки, `updated_at` не трогается.
-- Подписей к фото здесь нет: у всех фото alt_text пуст, переводить нечего.
-- Тексты в долларовых кавычках ($tr$ ... $tr$).

BEGIN;

-- granny-baba-yaga (ru)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'ru', $tr$Бабушка Баба-Яга$tr$, $tr$Маленькая авторская кукла — Баба-Яга. Вылеплена из полимерной глины, расписана акриловыми красками и одета вручную. Фигурка целиком ручной работы, в единственном экземпляре$tr$, $tr$Эта единственная в своём роде миниатюрная лесная ведьма сделана вручную: вылеплена и расписана с удивительно выразительными чертами лица, в которых читается хитрая, характерная улыбка. На седых волосах у неё уютный коричневый вязаный платок, под ним бежевая блузка, белая жилетка с пуговицами и тёмно-коричневый фартук, украшенный мшистой зеленью, поверх тёмной юбки.

В руке она держит подробно проработанную деревенскую метлу ручной работы из натурального дерева и шпагата. Внутри у неё проволочный каркас, поэтому руки и ноги можно слегка сгибать и менять позу.

Эта хрупкая коллекционная фигурка — не игрушка. Она станет очаровательным дополнением к сказочным диорамам, интерьерам в стиле коттеджкор и коллекциям миниатюр, вдохновлённых фольклором.$tr$, $tr$Акриловые краски, полимерная глина, ткань, шерсть$tr$, $tr$Лепка и роспись вручную, ручная работа$tr$
  FROM figurines WHERE slug = 'granny-baba-yaga'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- granny-baba-yaga (uk)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'uk', $tr$Бабуся Баба-Яга$tr$, $tr$Маленька авторська лялька: Баба-Яга. Виліплено з полімерної глини, розписано акриловими фарбами, одяг пошито вручну. Повністю ручна робота, фігурка в єдиному екземплярі$tr$, $tr$Ця єдина у своєму роді мініатюрна лісова відьма зроблена вручну: виліплена й розписана з напрочуд виразними рисами обличчя, у яких читається хитра, характерна посмішка. На сивому волоссі в неї затишна коричнева в’язана хустка, під нею бежева блузка, біла жилетка з ґудзиками та темно-коричневий фартух, прикрашений моховою зеленню, поверх темної спідниці.

У руці вона тримає детально опрацьовану сільську мітлу ручної роботи з натурального дерева й шпагату. Усередині дротяний каркас, тож руки й ноги можна злегка згинати, змінюючи позу.

Ця тендітна колекційна фігурка — не іграшка. Вона стане чудовим доповненням до казкових діорам, інтер’єрів у стилі котеджкор і колекцій мініатюр, натхненних фольклором.$tr$, $tr$Акрилова фарба, полімерна глина, тканина, вовна$tr$, $tr$Виліплено вручну, розписано вручну, ручна робота$tr$
  FROM figurines WHERE slug = 'granny-baba-yaga'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- granny-baba-yaga (es)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'es', $tr$Abuela Baba Yagá$tr$, $tr$Pequeña muñeca de autor: Baba Yagá. Modelada en arcilla polimérica, pintada con acrílicos y vestida a mano. Figura artística única, hecha por completo a mano$tr$, $tr$Esta bruja del bosque en miniatura, única en su especie, está modelada y pintada a mano, con unos rasgos faciales de una expresividad notable que captan una sonrisa pícara y llena de carácter. Sobre el pelo gris lleva un acogedor pañuelo de punto marrón; viste una blusa beige, un chaleco blanco con botones y, sobre una falda oscura, un delantal marrón oscuro adornado con un verdor que recuerda al musgo.

En la mano sostiene una escoba rústica y detallada, hecha a mano con madera natural y cordel. Gracias a un armazón interior de alambre, brazos y piernas se pueden colocar con suavidad para cambiar ligeramente la postura.

Esta delicada pieza de colección, que no es un juguete, es un complemento encantador para dioramas de cuentos de hadas, decoración cottagecore y colecciones de miniaturas inspiradas en el folclore.$tr$, $tr$Pintura acrílica, arcilla polimérica, tela, lana$tr$, $tr$Modelado a mano, pintado a mano, hecho a mano$tr$
  FROM figurines WHERE slug = 'granny-baba-yaga'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- granny-baba-yaga (de)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'de', $tr$Großmutter Baba Jaga$tr$, $tr$Kleine Künstlerpuppe: Baba Jaga. Aus Polymerton modelliert, mit Acrylfarben bemalt und von Hand angekleidet. Eine vollständig handgefertigte Figur, ein Unikat$tr$, $tr$Diese einzigartige, von Hand gefertigte Miniatur-Waldhexe ist von Hand modelliert und bemalt, mit auffallend ausdrucksvollen Gesichtszügen, in denen ein verschmitztes, charaktervolles Lächeln liegt. Über dem grauen Haar trägt sie ein gemütliches braunes Strickkopftuch, dazu eine beigefarbene Bluse, eine weiße Weste mit Knöpfen und über einem dunklen Rock eine dunkelbraune Schürze, die mit moosartigem Grün verziert ist.

In der Hand hält sie einen detailreichen, handgemachten, rustikalen Besen aus Naturholz und Garn. Dank eines Drahtgerüsts im Inneren lassen sich Arme und Beine behutsam bewegen und die Haltung leicht verändern.

Dieses zarte Sammlerstück ist kein Spielzeug. Es ist eine bezaubernde Ergänzung für Märchendioramen, Cottagecore-Einrichtung und von der Volkskunde inspirierte Miniatursammlungen.$tr$, $tr$Acrylfarbe, Polymerton, Stoff, Wolle$tr$, $tr$Von Hand modelliert, von Hand bemalt, handgefertigt$tr$
  FROM figurines WHERE slug = 'granny-baba-yaga'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- granny-baba-yaga (it)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'it', $tr$Nonna Baba Yaga$tr$, $tr$Piccola bambola d’autore: Baba Yaga. Modellata in argilla polimerica, dipinta con colori acrilici e vestita a mano. Figura d’arte interamente fatta a mano, pezzo unico$tr$, $tr$Questa strega del bosco in miniatura, un esemplare unico, è modellata e dipinta a mano, con tratti del viso straordinariamente espressivi che colgono un sorriso furbo e pieno di carattere. Sui capelli grigi porta un accogliente fazzoletto di maglia marrone; indossa una camicetta beige, un gilet bianco con bottoni e, sopra una gonna scura, un grembiule marrone scuro ornato da un verde che ricorda il muschio.

In mano tiene una scopa rustica e dettagliata, fatta a mano con legno naturale e spago. Grazie a un’armatura interna di filo metallico, braccia e gambe si possono muovere con delicatezza per cambiare leggermente la posa.

Questo delicato oggetto da collezione, che non è un giocattolo, è un’aggiunta incantevole per diorami di fiabe, arredi cottagecore e raccolte di miniature ispirate al folclore.$tr$, $tr$Colori acrilici, argilla polimerica, tessuto, lana$tr$, $tr$Modellatura a mano, pittura a mano, lavorazione artigianale$tr$
  FROM figurines WHERE slug = 'granny-baba-yaga'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- green-gnome-gardener (ru)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'ru', $tr$Зелёный гном-садовник$tr$, $tr$Маленькая авторская кукла — гном-садовник. Вылеплен из полимерной глины, расписан акриловыми красками и одет вручную. Фигурка целиком ручной работы, в единственном экземпляре$tr$, $tr$Этот единственный в своём роде миниатюрный гном-садовник целиком сделан вручную: вылеплен и расписан, с выразительным мягким лицом, острыми ушами и светлыми русыми волосами. На нём высокий остроконечный колпак цвета лесной зелени с ручной отделкой швами, оливковая туника с узором, зелёные штаны и тёмно-коричневый фартук с маленьким карманом, набитым сушёными травами. В правой руке у него крошечная лейка ручной работы: он готов ухаживать за лесными растениями.

Внутри гнома гибкий проволочный каркас, поэтому руки и ноги можно ставить так, как нужно для естественной позы. Ростом он около 7–8 см. Эта хрупкая коллекционная фигурка — не игрушка. Она станет милым дополнением к композициям в стиле коттеджкор, сказочным диорамам и коллекциям авторских кукол, вдохновлённых фольклором.$tr$, $tr$Акриловые краски, полимерная глина, ткань, шерсть$tr$, $tr$Лепка и роспись вручную, ручная работа$tr$
  FROM figurines WHERE slug = 'green-gnome-gardener'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- green-gnome-gardener (uk)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'uk', $tr$Зелений гном-садівник$tr$, $tr$Маленька авторська лялька: гном-садівник. Виліплено з полімерної глини, розписано акриловими фарбами, одяг пошито вручну. Повністю ручна робота, фігурка в єдиному екземплярі$tr$, $tr$Цей єдиний у своєму роді мініатюрний гном-садівник повністю зроблений вручну: виліплений і розписаний, з виразним лагідним обличчям, гострими вухами та м’яким русявим волоссям. На ньому високий гострий ковпак кольору лісової зелені з ручним оздобленням швами, оливкова туніка з візерунком, зелені штани й темно-коричневий фартух із маленькою кишенею, повною сушених трав. У правій руці в нього крихітна лійка ручної роботи: він готовий доглядати за лісовими рослинами.

Усередині гнома гнучкий дротяний каркас, тож руки й ноги можна ставити так, як потрібно для природної пози. Зростом він близько 7–8 см. Ця тендітна колекційна фігурка — не іграшка. Вона стане чарівним доповненням до композицій у стилі котеджкор, казкових діорам і колекцій авторських ляльок, натхненних фольклором.$tr$, $tr$Акрилова фарба, полімерна глина, тканина, вовна$tr$, $tr$Виліплено вручну, розписано вручну, ручна робота$tr$
  FROM figurines WHERE slug = 'green-gnome-gardener'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- green-gnome-gardener (es)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'es', $tr$Gnomo jardinero verde$tr$, $tr$Pequeña muñeca de autor: un gnomo jardinero. Modelada en arcilla polimérica, pintada con acrílicos y vestida a mano. Figura artística única, hecha por completo a mano$tr$, $tr$Este gnomo jardinero en miniatura, único en su especie, está modelado y pintado por completo a mano, con un rostro expresivo y amable, orejas puntiagudas y un suave pelo castaño claro. Lleva un alto gorro de tela puntiagudo en verde bosque, con detalles cosidos a mano, una túnica oliva estampada, pantalones verdes y un delantal marrón oscuro con un pequeño bolsillo lleno de hierbas secas. En la mano derecha sostiene una diminuta regadera hecha a mano, listo para cuidar de sus plantas del bosque.

Gracias a un armazón interior flexible de alambre, brazos y piernas se pueden colocar para lograr una postura natural. Mide unos 7–8 cm de altura. Esta delicada pieza de colección, que no es un juguete, es un complemento encantador para composiciones cottagecore, dioramas de cuentos de hadas y exposiciones de muñecas artísticas inspiradas en el folclore.$tr$, $tr$Pintura acrílica, arcilla polimérica, tela, lana$tr$, $tr$Modelado a mano, pintado a mano, hecho a mano$tr$
  FROM figurines WHERE slug = 'green-gnome-gardener'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- green-gnome-gardener (de)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'de', $tr$Grüner Gärtnergnom$tr$, $tr$Kleine Künstlerpuppe: Gärtnergnom. Aus Polymerton modelliert, mit Acrylfarben bemalt und von Hand angekleidet. Eine vollständig handgefertigte Figur, ein Unikat$tr$, $tr$Dieser einzigartige Miniatur-Gärtnergnom ist vollständig von Hand modelliert und bemalt, mit ausdrucksvollem, sanftem Gesicht, spitzen Ohren und weichem bräunlichem Haar. Er trägt eine hohe, spitze Stoffmütze in Waldgrün mit handgenähten Details, eine gemusterte olivgrüne Tunika, grüne Hosen und eine dunkelbraune Schürze mit einer kleinen Tasche voller getrockneter Kräuter. In der rechten Hand hält er eine winzige, handgefertigte Gießkanne, bereit, sich um seine Waldpflanzen zu kümmern.

Dank eines flexiblen Drahtgerüsts im Inneren lassen sich Arme und Beine für eine natürliche Haltung bewegen. Er ist etwa 7–8 cm hoch. Dieses zarte Sammlerstück ist kein Spielzeug. Es ist eine reizvolle Ergänzung für Cottagecore-Arrangements, Märchendioramen und von der Volkskunde inspirierte Künstlerpuppen-Sammlungen.$tr$, $tr$Acrylfarbe, Polymerton, Stoff, Wolle$tr$, $tr$Von Hand modelliert, von Hand bemalt, handgefertigt$tr$
  FROM figurines WHERE slug = 'green-gnome-gardener'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- green-gnome-gardener (it)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'it', $tr$Gnomo giardiniere verde$tr$, $tr$Piccola bambola d’autore: un gnomo giardiniere. Modellata in argilla polimerica, dipinta con colori acrilici e vestita a mano. Figura d’arte interamente fatta a mano, pezzo unico$tr$, $tr$Questo gnomo giardiniere in miniatura, un esemplare unico, è interamente modellato e dipinto a mano, con un viso espressivo e dolce, orecchie a punta e morbidi capelli castani. Indossa un alto cappello di stoffa a punta verde bosco, con rifiniture cucite a mano, una tunica oliva a disegni, pantaloni verdi e un grembiule marrone scuro con una piccola tasca piena di erbe secche. Nella mano destra tiene un minuscolo annaffiatoio fatto a mano, pronto a prendersi cura delle sue piante del bosco.

Grazie a un’armatura interna flessibile di filo metallico, braccia e gambe si possono mettere in posa in modo naturale. È alto circa 7–8 cm. Questo delicato oggetto da collezione, che non è un giocattolo, è un’aggiunta affascinante per composizioni cottagecore, diorami di fiabe ed esposizioni di bambole d’artista ispirate al folclore.$tr$, $tr$Colori acrilici, argilla polimerica, tessuto, lana$tr$, $tr$Modellatura a mano, pittura a mano, lavorazione artigianale$tr$
  FROM figurines WHERE slug = 'green-gnome-gardener'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- little-guy-with-pumpkins (ru)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'ru', $tr$Малыш с тыквами$tr$, $tr$Маленькая авторская кукла — гном, вырезающий тыквы. Вылеплен из полимерной глины, расписан акриловыми красками и одет вручную. Фигурка целиком ручной работы, в единственном экземпляре$tr$, $tr$Миниатюрный гном с тыквами — кукла ручной работы в единственном экземпляре (ростом около 10 см)

Знакомьтесь: проказливый маленький гном, герой прямо из осенней сказки! С высоким остроконечным колпаком и озорным выражением лица он похож на загадочного лесного странника, неравнодушного к тыквам. Он одет в тёмный жилет и фактурные коричневые штаны, в одной руке держит миниатюрную тыкву, в другой — крошечный нож. Ещё две тыквы довершают его очаровательную осеннюю сценку.

Эта миниатюрная кукла в единственном экземпляре тщательно сделана вручную, а лицо вылеплено и расписано вручную и полно характера. Насыщенные землистые цвета, фактурные ткани и мельчайшие детали придают ему деревенский, чуть волшебный вид. Ростом около 10 см, он станет небольшим, но заметным дополнением к любой коллекции миниатюр.

Он подойдёт собирателям уникальных авторских кукол, любителям гномов, фольклора и фэнтези и всем, кому нравятся лесные персонажи и осенний декор. Он будет хорошо смотреться в миниатюрной лесной сцене, сезонной композиции, кукольном домике или в коллекции необычных сказочных существ.

Каждая кукла делается вручную по отдельности, поэтому двух одинаковых не бывает. Это коллекционное произведение искусства, созданное для того, чтобы им любоваться, а не для детских игр.

Куклу аккуратно упакуют в защитную упаковку и отправят с трек-номером.$tr$, $tr$Акриловые краски, полимерная глина, ткань, шерсть$tr$, $tr$Лепка и роспись вручную, ручная работа$tr$
  FROM figurines WHERE slug = 'little-guy-with-pumpkins'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- little-guy-with-pumpkins (uk)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'uk', $tr$Малюк із гарбузами$tr$, $tr$Маленька авторська лялька: гном, що вирізає гарбузи. Виліплено з полімерної глини, розписано акриловими фарбами, одяг пошито вручну. Повністю ручна робота, фігурка в єдиному екземплярі$tr$, $tr$Мініатюрний гном із гарбузами — лялька ручної роботи в єдиному екземплярі (зростом близько 10 см)

Знайомтеся: пустотливий маленький гном, герой просто з осінньої казки! З високим гострим ковпаком і бешкетним виразом обличчя він схожий на загадкового лісового мандрівника, небайдужого до гарбузів. Він одягнений у темний жилет і фактурні коричневі штани, в одній руці тримає мініатюрний гарбуз, у другій — крихітний ніж. Ще два гарбузи доповнюють його чарівну осінню сценку.

Ця мініатюрна лялька в єдиному екземплярі ретельно зроблена вручну, а обличчя виліплене й розписане вручну та сповнене характеру. Насичені землисті кольори, фактурні тканини й найдрібніші деталі надають йому сільського, трохи чарівного вигляду. Зростом близько 10 см, він стане невеликим, але помітним доповненням до будь-якої колекції мініатюр.

Він підійде колекціонерам унікальних авторських ляльок, шанувальникам гномів, фольклору й фентезі та всім, кому подобаються лісові персонажі й осінній декор. Він чудово виглядатиме в мініатюрній лісовій сцені, сезонній композиції, ляльковому будиночку чи колекції незвичайних казкових істот.

Кожна лялька виготовляється вручну окремо, тому двох однакових не буває. Це колекційний твір мистецтва, створений для того, щоб ним милуватися, а не для дитячих ігор.

Ляльку дбайливо запакують у захисну упаковку й надішлють із номером для відстеження.$tr$, $tr$Акрилова фарба, полімерна глина, тканина, вовна$tr$, $tr$Виліплено вручну, розписано вручну, ручна робота$tr$
  FROM figurines WHERE slug = 'little-guy-with-pumpkins'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- little-guy-with-pumpkins (es)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'es', $tr$Pequeño con calabazas$tr$, $tr$Pequeña muñeca de autor: un gnomo tallando calabazas. Modelada en arcilla polimérica, pintada con acrílicos y vestida a mano. Figura artística única, hecha por completo a mano$tr$, $tr$Gnomo calabacero en miniatura: muñeca artística hecha a mano, pieza única (unos 10 cm / 4 pulgadas de altura)

Te presentamos a este pequeño gnomo travieso, un personaje salido de un cuento de otoño. Con su alto sombrero puntiagudo y su expresión descarada, parece un misterioso caminante del bosque con debilidad por las calabazas. Va vestido con una chaqueta oscura y unos pantalones marrones texturados; en una mano lleva una calabaza en miniatura y en la otra un diminuto cuchillo. Otras dos calabazas completan su encantadora escena otoñal.

Esta muñeca en miniatura, pieza única, está hecha con cuidado a mano, con un rostro modelado y pintado a mano, lleno de personalidad. Los ricos colores terrosos, las telas texturadas y los pequeños detalles le dan un carácter rústico y ligeramente mágico. Con unos 10 cm (4 pulgadas) de altura, es un añadido pequeño pero singular para cualquier colección de miniaturas.

Es ideal para coleccionistas de muñecas artísticas únicas, aficionados a los gnomos, amantes del folclore y la fantasía y para quien disfrute de los personajes del bosque y la decoración de otoño. Quedaría precioso en una escena de bosque en miniatura, una composición de temporada, una casa de muñecas o una colección de criaturas curiosas de cuento.

Cada muñeca se hace a mano de forma individual, así que no hay dos iguales. Es una obra de arte de colección, creada para ser expuesta y admirada, no para el juego infantil.

La muñeca se embalará con cuidado en un envoltorio protector y se enviará con número de seguimiento.$tr$, $tr$Pintura acrílica, arcilla polimérica, tela, lana$tr$, $tr$Modelado a mano, pintado a mano, hecho a mano$tr$
  FROM figurines WHERE slug = 'little-guy-with-pumpkins'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- little-guy-with-pumpkins (de)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'de', $tr$Kleiner Kerl mit Kürbissen$tr$, $tr$Kleine Künstlerpuppe: Gnom beim Kürbisschnitzen. Aus Polymerton modelliert, mit Acrylfarben bemalt und von Hand angekleidet. Eine vollständig handgefertigte Figur, ein Unikat$tr$, $tr$Miniatur-Kürbisgnom – handgefertigte Künstlerpuppe, ein Unikat (etwa 10 cm / 4 Zoll hoch)

Das ist ein schelmischer kleiner Gnom, eine Gestalt wie aus einem Herbstmärchen! Mit seinem hohen spitzen Hut und dem frechen Gesichtsausdruck sieht er aus wie ein geheimnisvoller Wanderer des Waldes mit einer Schwäche für Kürbisse. Er trägt eine dunkle Jacke und strukturierte braune Hosen, in der einen Hand hält er einen Miniaturkürbis, in der anderen ein winziges Messer. Zwei weitere Kürbisse runden seine reizende Herbstszene ab.

Diese Miniaturpuppe, ein Unikat, ist sorgfältig von Hand gefertigt, mit einem von Hand modellierten und bemalten Gesicht voller Persönlichkeit. Die satten Erdfarben, die strukturierten Stoffe und die kleinen Details geben ihm einen rustikalen, leicht magischen Charakter. Mit etwa 10 cm (4 Zoll) Höhe ist er eine kleine, aber unverwechselbare Ergänzung für jede Miniatursammlung.

Er passt zu Sammlern einzigartiger Künstlerpuppen, Gnomenfreunden, Liebhabern von Volkskunde und Fantasy und allen, die Waldgestalten und Herbstdekoration mögen. Er würde sich wunderbar in einer Miniatur-Waldszene, einer saisonalen Auslage, einem Puppenhaus oder einer Sammlung kurioser Märchenwesen machen.

Jede Puppe wird einzeln von Hand gefertigt, daher gleicht keine der anderen. Sie ist ein Kunstwerk für Sammler, geschaffen zum Ausstellen und Bewundern, nicht zum Spielen für Kinder.

Die Puppe wird sorgfältig in schützender Verpackung verpackt und mit Sendungsnummer verschickt.$tr$, $tr$Acrylfarbe, Polymerton, Stoff, Wolle$tr$, $tr$Von Hand modelliert, von Hand bemalt, handgefertigt$tr$
  FROM figurines WHERE slug = 'little-guy-with-pumpkins'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- little-guy-with-pumpkins (it)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'it', $tr$Ometto con le zucche$tr$, $tr$Piccola bambola d’autore: un gnomo che intaglia zucche. Modellata in argilla polimerica, dipinta con colori acrilici e vestita a mano. Figura d’arte interamente fatta a mano, pezzo unico$tr$, $tr$Gnomo con le zucche in miniatura – bambola d’artista fatta a mano, pezzo unico (alto circa 10 cm / 4 pollici)

Ecco un piccolo gnomo dispettoso, un personaggio uscito da una fiaba autunnale. Con l’alto cappello a punta e l’espressione sfrontata, sembra un misterioso viandante del bosco con un debole per le zucche. Indossa una giacca scura e pantaloni marroni materici; in una mano porta una zucca in miniatura e nell’altra un minuscolo coltello. Altre due zucche completano la sua deliziosa scena autunnale.

Questa bambola in miniatura, un pezzo unico, è fatta con cura a mano, con un viso modellato e dipinto a mano e pieno di personalità. I ricchi colori terrosi, i tessuti materici e i piccoli dettagli gli danno un carattere rustico e leggermente magico. Alto circa 10 cm (4 pollici), è un’aggiunta piccola ma inconfondibile a qualsiasi collezione di miniature.

È ideale per chi colleziona bambole d’artista uniche, per gli appassionati di gnomi, per chi ama il folclore e il fantasy e per chiunque apprezzi i personaggi del bosco e le decorazioni autunnali. Starebbe benissimo in una scena di bosco in miniatura, in un’esposizione stagionale, in una casa delle bambole o in una raccolta di strane creature delle fiabe.

Ogni bambola è fatta a mano singolarmente, perciò non ce ne sono due uguali. È un’opera d’arte da collezione, creata per essere esposta e ammirata, non per il gioco dei bambini.

La bambola viene imballata con cura in una confezione protettiva e spedita con numero di tracciamento.$tr$, $tr$Colori acrilici, argilla polimerica, tessuto, lana$tr$, $tr$Modellatura a mano, pittura a mano, lavorazione artigianale$tr$
  FROM figurines WHERE slug = 'little-guy-with-pumpkins'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- old-lady-gnome (ru)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'ru', $tr$Старая гномиха$tr$, $tr$Маленькая авторская кукла — гномиха. Вылеплена из полимерной глины, расписана акриловыми красками и одета вручную. Фигурка целиком ручной работы, в единственном экземпляре$tr$, $tr$Миниатюрная лесная ведьма с посохом и ключом — авторская кукла ручной работы в единственном экземпляре (ростом около 9–10 см)

Знакомьтесь: загадочная маленькая лесная ведьма, будто сошедшая со страниц древней лесной сказки. Выразительное лицо, длинные седовато-каштановые косы и особенный чёрный плащ с капюшоном придают ей спокойную мудрость того, кто знает тайны леса. Землисто-коричневая кофта, глубокого красного цвета юбка и крошечные декоративные детали придают ей деревенский, вдохновлённый фольклором облик.

В руке у неё натуральный деревянный посох, с которым она идёт по заколдованному лесу, а на шее висит затейливый золотой ключ: быть может, он отпирает давно забытую дверь или хранит тайну, известную только ей. Тёмный плащ, красная строчка и тщательно уложенные слоями ткани делают её на редкость фактурной, чуть таинственной и полной характера.

Ростом около 9–10 см, эта миниатюрная авторская кукла в единственном экземпляре сделана вручную: лицо вылеплено и расписано вручную, а детали из ткани и шерсти аккуратно подобраны. Каждая деталь работает на её неповторимый характер, и она — маленькое, но завораживающее произведение рукотворного фэнтези.

Она станет прекрасным дополнением к коллекции авторских кукол, ведьм, фольклорных персонажей и миниатюрных фэнтезийных фигурок. Она также подойдёт для лесных диорам, мрачных сказочных композиций, готических коллекций, миниатюрных сцен и полок, на которых стоят необычные вещицы ручной работы.$tr$, $tr$Акриловые краски, полимерная глина, ткань, шерсть$tr$, $tr$Лепка и роспись вручную, ручная работа$tr$
  FROM figurines WHERE slug = 'old-lady-gnome'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- old-lady-gnome (uk)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'uk', $tr$Стара гномиха$tr$, $tr$Маленька авторська лялька: гномиха. Виліплено з полімерної глини, розписано акриловими фарбами, одяг пошито вручну. Повністю ручна робота, фігурка в єдиному екземплярі$tr$, $tr$Мініатюрна лісова відьма з посохом і ключем — авторська лялька ручної роботи в єдиному екземплярі (зростом близько 9–10 см)

Знайомтеся: загадкова маленька лісова відьма, ніби зійшла зі сторінок давньої лісової казки. Виразне обличчя, довгі сивувато-каштанові коси й особливий чорний плащ із каптуром надають їй спокійної мудрості того, хто знає таємниці лісу. Землисто-коричнева кофта, глибоко-червона спідниця й крихітні декоративні деталі надають їй сільського, натхненного фольклором вигляду.

У руці вона тримає натуральний дерев’яний посох, з яким іде зачарованим лісом, а на шиї в неї висить химерний золотий ключ: можливо, він відмикає давно забуті двері або береже таємницю, відому лише їй. Темний плащ, червоне зшивання й ретельно укладені шарами тканини роблять її напрочуд фактурною, трохи таємничою й повною характеру.

Зростом близько 9–10 см, ця мініатюрна авторська лялька в єдиному екземплярі зроблена вручну: обличчя виліплене й розписане вручну, а деталі з тканини та вовни дбайливо підібрані. Кожна деталь працює на її неповторний характер, і вона — маленький, але захопливий твір рукотворного фентезі.

Вона стане чудовим доповненням до колекції авторських ляльок, відьом, фольклорних персонажів і мініатюрних фентезі-фігурок. Вона також підійде для лісових діорам, похмурих казкових композицій, готичних колекцій, мініатюрних сцен і полиць, на яких стоять незвичайні речі ручної роботи.$tr$, $tr$Акрилова фарба, полімерна глина, тканина, вовна$tr$, $tr$Виліплено вручну, розписано вручну, ручна робота$tr$
  FROM figurines WHERE slug = 'old-lady-gnome'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- old-lady-gnome (es)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'es', $tr$Anciana gnoma$tr$, $tr$Pequeña muñeca de autor: una gnoma. Modelada en arcilla polimérica, pintada con acrílicos y vestida a mano. Figura artística única, hecha por completo a mano$tr$, $tr$Bruja del bosque en miniatura con bastón y llave: muñeca artística hecha a mano, pieza única (unos 9–10 cm / 3,5–4 pulgadas de altura)

Te presentamos a esta pequeña y misteriosa bruja del bosque, un personaje que parece salido de un antiguo cuento de bosques. Con su rostro expresivo, su larga trenza gris pardo y su inconfundible capa negra con capucha, transmite la sabiduría tranquila de quien conoce los secretos del bosque. Su blusa marrón terrosa, su falda de un rojo profundo y los pequeños detalles decorativos le dan un aire rústico, inspirado en el folclore.

Sostiene un bastón de madera natural mientras recorre bosques encantados, y del cuello le cuelga una llave dorada con adornos, que quizá abra una puerta olvidada o guarde un secreto que solo ella conoce. La capa oscura, las costuras rojas y las telas cuidadosamente superpuestas crean un personaje maravillosamente texturado, algo misterioso y lleno de personalidad.

Con unos 9–10 cm (3,5–4 pulgadas) de altura, esta muñeca artística en miniatura, pieza única, está hecha a mano, con un rostro modelado y pintado a mano y detalles de tela y lana cuidadosamente dispuestos. Cada elemento contribuye a su carácter singular y la convierte en una pequeña pero cautivadora obra de fantasía artesanal.

Será un añadido precioso para una colección de muñecas artísticas, brujas, personajes del folclore y figuras de fantasía en miniatura. También es ideal para dioramas de bosque, escenas de cuentos oscuros, colecciones góticas, escenas en miniatura y estantes llenos de tesoros artesanales poco comunes.$tr$, $tr$Pintura acrílica, arcilla polimérica, tela, lana$tr$, $tr$Modelado a mano, pintado a mano, hecho a mano$tr$
  FROM figurines WHERE slug = 'old-lady-gnome'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- old-lady-gnome (de)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'de', $tr$Alte Gnomenfrau$tr$, $tr$Kleine Künstlerpuppe: Gnomenfrau. Aus Polymerton modelliert, mit Acrylfarben bemalt und von Hand angekleidet. Eine vollständig handgefertigte Figur, ein Unikat$tr$, $tr$Miniatur-Waldhexe mit Stab und Schlüssel – handgefertigte Künstlerpuppe, ein Unikat (etwa 9–10 cm / 3,5–4 Zoll hoch)

Das ist eine geheimnisvolle kleine Waldhexe, eine Gestalt wie aus einem alten Waldmärchen. Mit ihrem ausdrucksvollen Gesicht, dem langen geflochtenen graubraunen Haar und dem unverwechselbaren schwarzen Kapuzenumhang strahlt sie die stille Weisheit dessen aus, der die Geheimnisse des Waldes kennt. Ihr erdbraunes Oberteil, der tiefrote Rock und die kleinen dekorativen Details geben ihr ein rustikales, von der Volkskunde inspiriertes Aussehen.

Sie hält einen Stab aus Naturholz, während sie durch verzauberte Wälder zieht, und um ihren Hals hängt ein verzierter goldener Schlüssel, der vielleicht eine vergessene Tür öffnet oder ein Geheimnis hütet, das nur sie kennt. Der dunkle Umhang, die roten Nähte und die sorgfältig geschichteten Stoffe machen sie zu einer wunderbar strukturierten, leicht geheimnisvollen Gestalt voller Persönlichkeit.

Mit etwa 9–10 cm (3,5–4 Zoll) Höhe ist diese Miniatur-Künstlerpuppe, ein Unikat, einzeln von Hand gefertigt, mit einem von Hand modellierten und bemalten Gesicht und sorgfältig angeordneten Stoff- und Wolldetails. Jedes Element trägt zu ihrem einzigartigen Charakter bei und macht sie zu einem winzigen, aber fesselnden Stück handgemachter Fantasy-Kunst.

Sie ist eine schöne Ergänzung für eine Sammlung von Künstlerpuppen, Hexen, Gestalten der Volkskunde und Fantasy-Miniaturfiguren. Sie passt auch zu Walddioramen, dunklen Märchenszenen, Gotik-Sammlungen, Miniaturszenen und Regalen voller ungewöhnlicher handgemachter Schätze.$tr$, $tr$Acrylfarbe, Polymerton, Stoff, Wolle$tr$, $tr$Von Hand modelliert, von Hand bemalt, handgefertigt$tr$
  FROM figurines WHERE slug = 'old-lady-gnome'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- old-lady-gnome (it)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'it', $tr$Vecchia gnoma$tr$, $tr$Piccola bambola d’autore: una gnoma. Modellata in argilla polimerica, dipinta con colori acrilici e vestita a mano. Figura d’arte interamente fatta a mano, pezzo unico$tr$, $tr$Strega del bosco in miniatura con bastone e chiave – bambola d’artista fatta a mano, pezzo unico (alta circa 9–10 cm / 3,5–4 pollici)

Ecco una misteriosa piccola strega del bosco, un personaggio che sembra uscito da un’antica fiaba di foresta. Con il viso espressivo, le lunghe trecce grigio-castane e il caratteristico mantello nero con cappuccio, ha la saggezza tranquilla di chi conosce i segreti del bosco. La blusa marrone terra, la gonna rosso intenso e i piccoli dettagli decorativi le danno un aspetto rustico, ispirato al folclore.

Regge un bastone di legno naturale mentre cammina per boschi incantati, e al collo le pende una chiave dorata e ornata, che forse apre una porta dimenticata o custodisce un segreto noto solo a lei. Il mantello scuro, le cuciture rosse e i tessuti accuratamente sovrapposti ne fanno un personaggio meravigliosamente materico, un po’ misterioso e pieno di personalità.

Alta circa 9–10 cm (3,5–4 pollici), questa bambola d’artista in miniatura, un pezzo unico, è fatta a mano singolarmente, con un viso modellato e dipinto a mano e dettagli di stoffa e lana disposti con cura. Ogni elemento contribuisce al suo carattere unico e ne fa una piccola ma affascinante opera di fantasy artigianale.

Sarà una bella aggiunta a una collezione di bambole d’artista, streghe, personaggi del folclore e figure fantasy in miniatura. È perfetta anche per diorami del bosco, scene di fiabe oscure, collezioni gotiche, scene in miniatura e scaffali pieni di insoliti tesori fatti a mano.$tr$, $tr$Colori acrilici, argilla polimerica, tessuto, lana$tr$, $tr$Modellatura a mano, pittura a mano, lavorazione artigianale$tr$
  FROM figurines WHERE slug = 'old-lady-gnome'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- quaismodo-doll (ru)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'ru', $tr$Кукла Квазимодо$tr$, $tr$Маленькая авторская кукла — Квазимодо. Вылеплен из полимерной глины, расписан акриловыми красками и одет вручную. Фигурка целиком ручной работы, в единственном экземпляре$tr$, $tr$Эта единственная в своём роде миниатюрная скульптура ручной работы изображает Квазимодо, знаменитого горбуна из Нотр-Дама, задумчиво сидящего на небольшой тёмной деревянной скамье. Он целиком вылеплен и расписан с тщательной проработкой лица: выразительное, трогательное лицо с мягкими рыжеватыми волосами и добрым, проникновенным взглядом.

Он одет в простую бежевую тунику с капюшоном, подпоясанную шпагатом, и полосатые тёмные штаны. На шее у него металлическая цепочка со старинными бронзовыми ключами, словно напоминающая о соборе. Внутри гибкий проволочный каркас, поэтому руки и ноги можно ставить в естественные, характерные позы.

Эта хрупкая коллекционная фигурка — не игрушка. Она станет эффектным дополнением к собраниям литературного искусства, витринам в стиле «тёмной академии», сказочным диорамам и шкафчикам с редкостями.$tr$, $tr$Акриловые краски, полимерная глина, ткань, шерсть$tr$, $tr$Лепка и роспись вручную, ручная работа$tr$
  FROM figurines WHERE slug = 'quaismodo-doll'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- quaismodo-doll (uk)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'uk', $tr$Лялька Квазімодо$tr$, $tr$Маленька авторська лялька: Квазімодо. Виліплено з полімерної глини, розписано акриловими фарбами, одяг пошито вручну. Повністю ручна робота, фігурка в єдиному екземплярі$tr$, $tr$Ця єдина у своєму роді мініатюрна скульптура ручної роботи зображує Квазімодо, знаменитого горбаня з Нотр-Даму, що задумливо сидить на невеликій темній дерев’яній лаві. Він повністю виліплений і розписаний із ретельним опрацюванням обличчя: виразне, зворушливе обличчя з м’яким рудуватим волоссям і добрим, проникливим поглядом.

Він одягнений у просту бежеву туніку з каптуром, підперезану шпагатом, і смугасті темні штани. На шиї в нього металевий ланцюжок зі старовинними бронзовими ключами, що ніби нагадує про собор. Усередині гнучкий дротяний каркас, тож руки й ноги можна ставити в природні, характерні пози.

Ця тендітна колекційна фігурка — не іграшка. Вона стане яскравим доповненням до зібрань літературного мистецтва, вітрин у стилі темної академії, казкових діорам і вітрин із дивовижами.$tr$, $tr$Акрилова фарба, полімерна глина, тканина, вовна$tr$, $tr$Виліплено вручну, розписано вручну, ручна робота$tr$
  FROM figurines WHERE slug = 'quaismodo-doll'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- quaismodo-doll (es)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'es', $tr$Muñeca Quasimodo$tr$, $tr$Pequeña muñeca de autor: Quasimodo. Modelada en arcilla polimérica, pintada con acrílicos y vestida a mano. Figura artística única, hecha por completo a mano$tr$, $tr$Esta escultura en miniatura, única en su especie y hecha a mano, representa a Quasimodo, el célebre jorobado de Notre Dame, sentado con aire pensativo en un pequeño banco de madera oscura. Está modelado y pintado por completo a mano, con un meticuloso trabajo del rostro: una cara expresiva y conmovedora, de pelo suave y rojizo y mirada dulce y profunda.

Viste una sencilla túnica beige con capucha, atada a la cintura con un cordel, y pantalones oscuros a rayas. Del cuello le cuelga una cadena metálica con llaves de bronce de aire antiguo, que añade un toque de leyenda de catedral a su postura. Gracias a un armazón interior flexible de alambre, brazos y piernas se pueden colocar en posturas naturales y llenas de carácter.

Esta delicada pieza de colección, que no es un juguete, es un complemento llamativo para colecciones de arte literario, estanterías de estética académica oscura, dioramas de cuentos y gabinetes de curiosidades.$tr$, $tr$Pintura acrílica, arcilla polimérica, tela, lana$tr$, $tr$Modelado a mano, pintado a mano, hecho a mano$tr$
  FROM figurines WHERE slug = 'quaismodo-doll'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- quaismodo-doll (de)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'de', $tr$Quasimodo-Puppe$tr$, $tr$Kleine Künstlerpuppe: Quasimodo. Aus Polymerton modelliert, mit Acrylfarben bemalt und von Hand angekleidet. Eine vollständig handgefertigte Figur, ein Unikat$tr$, $tr$Diese einzigartige, handgefertigte Miniaturskulptur zeigt Quasimodo, den berühmten Glöckner von Notre-Dame, der nachdenklich auf einer kleinen dunklen Holzbank sitzt. Er ist vollständig von Hand modelliert und bemalt, mit sorgfältig ausgearbeitetem Gesicht: ein ausdrucksvolles, ergreifendes Gesicht mit weichem rötlichem Haar und einem sanften, seelenvollen Blick.

Er trägt eine schlichte beigefarbene Kapuzentunika, in der Taille mit einer einfachen Schnur gebunden, und gestreifte dunkle Hosen. Um den Hals hängt eine Metallkette mit Schlüsseln aus Bronze im alten Stil, die seiner Haltung einen Hauch von Kathedralenlegende gibt. Dank eines flexiblen Drahtgerüsts im Inneren lassen sich Arme und Beine in natürliche, charaktervolle Haltungen bringen.

Dieses zarte Sammlerstück ist kein Spielzeug. Es ist eine eindrucksvolle Ergänzung für Sammlungen literarischer Kunst, Regale im Dark-Academia-Stil, Märchendioramen und Kuriositätenvitrinen.$tr$, $tr$Acrylfarbe, Polymerton, Stoff, Wolle$tr$, $tr$Von Hand modelliert, von Hand bemalt, handgefertigt$tr$
  FROM figurines WHERE slug = 'quaismodo-doll'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- quaismodo-doll (it)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'it', $tr$Bambola Quasimodo$tr$, $tr$Piccola bambola d’autore: Quasimodo. Modellata in argilla polimerica, dipinta con colori acrilici e vestita a mano. Figura d’arte interamente fatta a mano, pezzo unico$tr$, $tr$Questa scultura in miniatura, un esemplare unico fatto a mano, raffigura Quasimodo, il celebre gobbo di Notre-Dame, seduto pensieroso su una piccola panca di legno scuro. È interamente modellato e dipinto a mano, con il viso curato nei minimi dettagli: un volto espressivo e toccante, con morbidi capelli rossicci e uno sguardo dolce e profondo.

Indossa una semplice tunica beige con cappuccio, legata in vita con uno spago, e pantaloni scuri a righe. Al collo porta una catena metallica con chiavi di bronzo dall’aspetto antico, che aggiunge un tocco di leggenda della cattedrale alla sua posa. Grazie a un’armatura interna flessibile di filo metallico, braccia e gambe si possono mettere in pose naturali e piene di carattere.

Questo delicato oggetto da collezione, che non è un giocattolo, è un’aggiunta di grande effetto per raccolte di arte letteraria, scaffali dark academia, diorami di fiabe e vetrine di curiosità.$tr$, $tr$Colori acrilici, argilla polimerica, tessuto, lana$tr$, $tr$Modellatura a mano, pittura a mano, lavorazione artigianale$tr$
  FROM figurines WHERE slug = 'quaismodo-doll'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- tiny-with-with-broom (ru)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'ru', $tr$Крошка-ведьма с метлой$tr$, $tr$Маленькая авторская кукла — ведьма. Вылеплена из полимерной глины, расписана акриловыми красками и одета вручную. Фигурка целиком ручной работы, в единственном экземпляре$tr$, $tr$Эта единственная в своём роде миниатюрная лесная ведьма ростом около 10 см тщательно вылеплена и расписана вручную, с выразительными, характерными чертами лица. Лицо обрамляет буйная рыжевато-каштановая грива под классической остроконечной чёрной шляпой из ткани, а на шее лежит латунная подвеска-ключ в форме сердца в старинном стиле.

Она одета вручную: многослойное тёмно-коричневое платье-туника и накинутый чёрный плащ. В руке у неё подробно проработанная метла ручной работы из натурального дерева и шпагата. Внутри проволочный каркас, поэтому руки и ноги можно слегка сгибать. Она станет универсальным украшением для сцен кукольных домиков, шкафчиков с редкостями, готических библиотечных витрин или коллекций авторских кукол, вдохновлённых фольклором. Каждая фигурка делается вручную по отдельности и остаётся неповторимой хрупкой коллекционной вещью.$tr$, $tr$Акриловые краски, полимерная глина, ткань, шерсть$tr$, $tr$Лепка и роспись вручную, ручная работа$tr$
  FROM figurines WHERE slug = 'tiny-with-with-broom'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- tiny-with-with-broom (uk)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'uk', $tr$Крихітна відьма з мітлою$tr$, $tr$Маленька авторська лялька: відьма. Виліплено з полімерної глини, розписано акриловими фарбами, одяг пошито вручну. Повністю ручна робота, фігурка в єдиному екземплярі$tr$, $tr$Ця єдина у своєму роді мініатюрна лісова відьма зростом близько 10 см ретельно виліплена й розписана вручну, з виразними, характерними рисами обличчя. Обличчя обрамляє буйна рудувато-каштанова грива під класичним гострим чорним капелюхом із тканини, а на шиї лежить латунний кулон-ключ у формі серця в старовинному стилі.

Вона одягнена вручну: багатошарова темно-коричнева сукня-туніка й накинутий чорний плащ. У руці в неї детально опрацьована мітла ручної роботи з натурального дерева й шпагату. Усередині дротяний каркас, тож руки й ноги можна злегка згинати. Вона стане універсальною прикрасою для сцен лялькових будиночків, вітрин із дивовижами, готичних бібліотечних вітрин чи колекцій авторських ляльок, натхненних фольклором. Кожну фігурку виготовляють вручну окремо, і вона лишається неповторною тендітною колекційною річчю.$tr$, $tr$Акрилова фарба, полімерна глина, тканина, вовна$tr$, $tr$Виліплено вручну, розписано вручну, ручна робота$tr$
  FROM figurines WHERE slug = 'tiny-with-with-broom'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- tiny-with-with-broom (es)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'es', $tr$Pequeña bruja con escoba$tr$, $tr$Pequeña muñeca de autor: una bruja. Modelada en arcilla polimérica, pintada con acrílicos y vestida a mano. Figura artística única, hecha por completo a mano$tr$, $tr$Esta bruja del bosque en miniatura, única en su especie, mide unos 10 cm y está modelada y pintada a mano con esmero, con rasgos faciales expresivos y llenos de carácter. Enmarca su rostro una salvaje melena castaño rojiza bajo un clásico sombrero puntiagudo de tela negra, y del cuello le cuelga un colgante de latón con forma de llave-corazón, de estilo antiguo.

Está vestida a mano con un vestido túnica marrón oscuro en capas y una capa negra echada sobre los hombros, y sostiene una escoba detallada, hecha a mano con madera natural y cordel. Gracias a un armazón interior de alambre, brazos y piernas se pueden colocar con suavidad, lo que la convierte en una pieza versátil para escenas de casas de muñecas en miniatura, gabinetes de curiosidades, estanterías de biblioteca gótica o colecciones de muñecas artísticas inspiradas en el folclore. Cada figura se hace a mano de forma individual, como una pieza de colección única y delicada.$tr$, $tr$Pintura acrílica, arcilla polimérica, tela, lana$tr$, $tr$Modelado a mano, pintado a mano, hecho a mano$tr$
  FROM figurines WHERE slug = 'tiny-with-with-broom'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- tiny-with-with-broom (de)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'de', $tr$Winzige Hexe mit Besen$tr$, $tr$Kleine Künstlerpuppe: Hexe. Aus Polymerton modelliert, mit Acrylfarben bemalt und von Hand angekleidet. Eine vollständig handgefertigte Figur, ein Unikat$tr$, $tr$Diese einzigartige Miniatur-Waldhexe ist etwa 10 cm hoch, sorgfältig von Hand modelliert und bemalt, mit ausdrucksvollen, charaktervollen Gesichtszügen. Ihr Gesicht rahmt eine wilde rotbraune Haarmähne unter einem klassischen spitzen schwarzen Stoffhut, und um den Hals trägt sie einen Anhänger aus Messing, einen Herzschlüssel im alten Stil.

Sie ist von Hand gekleidet: ein mehrlagiges dunkelbraunes Tunikakleid und ein umgelegter schwarzer Umhang. In der Hand hält sie einen detailreichen, handgemachten Besen aus Naturholz und Garn. Dank eines Drahtgerüsts im Inneren lassen sich Arme und Beine behutsam bewegen, sodass sie sich vielseitig für Puppenhausszenen, Kuriositätenvitrinen, gotische Bibliotheksauslagen oder von der Volkskunde inspirierte Künstlerpuppen-Sammlungen eignet. Jede Figur wird einzeln von Hand gefertigt, als einzigartiges, zartes Sammlerstück.$tr$, $tr$Acrylfarbe, Polymerton, Stoff, Wolle$tr$, $tr$Von Hand modelliert, von Hand bemalt, handgefertigt$tr$
  FROM figurines WHERE slug = 'tiny-with-with-broom'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

-- tiny-with-with-broom (it)
INSERT INTO figurine_translations (figurine_id, lang, name, short_text, full_description, material, technique)
SELECT id, 'it', $tr$Piccola strega con la scopa$tr$, $tr$Piccola bambola d’autore: una strega. Modellata in argilla polimerica, dipinta con colori acrilici e vestita a mano. Figura d’arte interamente fatta a mano, pezzo unico$tr$, $tr$Questa strega del bosco in miniatura, un esemplare unico alto circa 10 cm, è modellata e dipinta a mano con cura, con tratti del viso espressivi e pieni di carattere. Il viso è incorniciato da una selvaggia chioma castano ramato sotto un classico cappello nero a punta di stoffa, e al collo porta un ciondolo di ottone a forma di chiave-cuore, in stile antico.

È vestita a mano con un abito-tunica marrone scuro a strati e un mantello nero drappeggiato, e tiene una scopa dettagliata, fatta a mano con legno naturale e spago. Grazie a un’armatura interna di filo metallico, braccia e gambe si possono mettere in posa con delicatezza: è perciò un pezzo versatile per scene di case delle bambole in miniatura, vetrine di curiosità, esposizioni di biblioteca gotica o raccolte di bambole d’artista ispirate al folclore. Ogni figura è fatta a mano singolarmente, come un delicato oggetto da collezione unico.$tr$, $tr$Colori acrilici, argilla polimerica, tessuto, lana$tr$, $tr$Modellatura a mano, pittura a mano, lavorazione artigianale$tr$
  FROM figurines WHERE slug = 'tiny-with-with-broom'
ON CONFLICT (figurine_id, lang) DO UPDATE
   SET name = COALESCE(NULLIF(figurine_translations.name, ''), EXCLUDED.name),
       short_text = COALESCE(NULLIF(figurine_translations.short_text, ''), EXCLUDED.short_text),
       full_description = COALESCE(NULLIF(figurine_translations.full_description, ''), EXCLUDED.full_description),
       material = COALESCE(NULLIF(figurine_translations.material, ''), EXCLUDED.material),
       technique = COALESCE(NULLIF(figurine_translations.technique, ''), EXCLUDED.technique);

COMMIT;

-- Проверка: должно вернуть 30 строк (6 работ × 5 языков), ни одной с пустым именем.
SELECT f.slug, t.lang, char_length(t.name) AS name_len, char_length(t.full_description) AS desc_len
  FROM figurine_translations t JOIN figurines f ON f.id = t.figurine_id
 WHERE f.slug IN ('granny-baba-yaga','green-gnome-gardener','little-guy-with-pumpkins',
                  'old-lady-gnome','quaismodo-doll','tiny-with-with-broom')
 ORDER BY f.slug, t.lang;
