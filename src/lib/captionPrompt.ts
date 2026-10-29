/**
 * Подпись для поиска: задание для модели и проверка того, что она прислала.
 *
 * Подпись — скрытый текст работы (`visual_caption`), посетителям он не виден.
 * Поиск читает его целыми словами: работа находится, если в подписи есть слово,
 * которое набрал посетитель, на его языке. Поэтому подпись состоит из двух строк:
 * описание того, что видно на фото, и строка `Keywords:` со словами на шести
 * языках, где у каждого предмета есть и название («axe»), и категория («weapon»).
 *
 * Текст задания лежит только здесь: панель «Подписи для поиска» кладёт его в
 * буфер обмена, подставив название и описание работы.
 */

/** Язык, порядок и число списков в строке Keywords. */
export const CAPTION_LANGS = ['EN', 'RU', 'UK', 'ES', 'DE', 'IT'] as const;

const EXAMPLE_GNOME = `A gnome with a long white beard bends forward and holds an axe with a brown wooden handle and a metal head. He wears a red cap with white dots, a houndstooth vest, a beige shirt, red trousers and brown shoes. Soft light gives a rustic, weathered mood.
Keywords: gnome, axe, weapon, tool, cap, red cap, white beard, beard, vest, houndstooth vest, shirt, trousers, shoes, bent forward, holding an axe, rustic, weathered, soft light, fairy tale character, old man, wooden handle, axe head, polka dots, long beard, metal | гном, топор, оружие, инструмент, шапка, красная шапка, белая борода, борода, жилет, жилет в гусиную лапку, рубашка, штаны, ботинки, наклонился вперёд, держит топор, деревенский, потёртый, мягкий свет, сказочный персонаж, старик, деревянная рукоять, лезвие топора, горошек, длинная борода, металл, секира | гном, сокира, зброя, інструмент, шапка, червона шапка, біла борода, борода, жилет, жилет у гусячу лапку, сорочка, штани, черевики, нахилився вперед, тримає сокиру, сільський, пошарпаний, м'яке світло, казковий персонаж, старий, дерев'яне руків'я, лезо сокири, горошок, довга борода, метал | gnomo, hacha, arma, herramienta, gorro, gorro rojo, barba blanca, barba, chaleco, chaleco de pata de gallo, camisa, pantalones, zapatos, inclinado hacia delante, sostiene un hacha, rústico, desgastado, luz suave, personaje de cuento, anciano, mango de madera, hoja del hacha, lunares, barba larga, metal | Gnom, Axt, Waffe, Werkzeug, Mütze, rote Mütze, weißer Bart, Bart, Weste, Weste mit Hahnentrittmuster, Hemd, Hose, Schuhe, nach vorn gebeugt, hält eine Axt, ländlich, abgenutzt, weiches Licht, Märchenfigur, alter Mann, Holzstiel, Axtblatt, Tupfen, langer Bart, Metall | gnomo, ascia, arma, strumento, berretto, berretto rosso, barba bianca, barba, gilet, gilet pied-de-poule, camicia, pantaloni, scarpe, piegato in avanti, impugna un'ascia, rustico, consumato, luce morbida, personaggio di fiaba, vecchio, manico di legno, lama dell'ascia, pois, barba lunga, metallo`;

const EXAMPLE_GRANNY = `Baba Yaga, an old woman with a stern, wrinkled face and grey hair under a brown woven headscarf, stands holding a tall broom made of a crooked branch with straw bristles. She wears a beige blouse, a white fur vest, a brown skirt of coarse cloth with a tuft of green moss, a dark underskirt and dark shoes. A calm, evenly lit photo with a white floral wallpaper and a dark table.
Keywords: Baba Yaga, witch, old woman, granny, grandmother, folklore character, standing, holding a broom, broom, tool, stick, straw, headscarf, scarf, grey hair, wrinkled face, stern, grumpy, fur vest, vest, blouse, skirt, shoes, green moss, moss, plant, floral wallpaper, rustic, folk tale, brown, beige, white, black, green | Баба-Яга, ведьма, пожилая женщина, бабушка, бабуля, персонаж фольклора, стоит, держит метлу, метла, инструмент, палка, солома, платок, косынка, седые волосы, морщинистое лицо, суровая, ворчливая, меховая жилетка, жилет, блузка, юбка, туфли, зелёный мох, мох, растение, цветочные обои, деревенский, сказочный, коричневый, бежевый, белый, чёрный, зелёный | Баба-Яга, відьма, літня жінка, бабуся, бабуня, персонаж фольклору, стоїть, тримає мітлу, мітла, інструмент, палиця, солома, хустка, косинка, сиве волосся, зморшкувате обличчя, сувора, буркотлива, хутряна жилетка, жилет, блузка, спідниця, туфлі, зелений мох, мох, рослина, квіткові шпалери, сільський, казковий, коричневий, бежевий, білий, чорний, зелений | Baba Yaga, bruja, anciana, abuela, abuelita, personaje del folclore, de pie, sostiene una escoba, escoba, herramienta, palo, paja, pañuelo, pañuelo de cabeza, pelo gris, rostro arrugado, severa, gruñona, chaleco de piel, chaleco, blusa, falda, zapatos, musgo verde, musgo, planta, papel pintado de flores, rústico, cuento popular, marrón, beige, blanco, negro, verde | Baba Jaga, Hexe, alte Frau, Oma, Großmutter, Folklorefigur, stehend, hält einen Besen, Besen, Werkzeug, Stock, Stroh, Kopftuch, Tuch, graue Haare, faltiges Gesicht, streng, mürrisch, Pelzweste, Weste, Bluse, Rock, Schuhe, grünes Moos, Moos, Pflanze, Blumentapete, rustikal, Märchen, braun, beige, weiß, schwarz, grün | Baba Yaga, strega, vecchia, nonna, nonnina, personaggio folkloristico, in piedi, tiene una scopa, scopa, attrezzo, bastone, paglia, fazzoletto, foulard, capelli grigi, viso rugoso, severa, scontrosa, gilet di pelliccia, gilet, camicetta, gonna, scarpe, muschio verde, muschio, pianta, carta da parati a fiori, rustico, fiaba, marrone, beige, bianco, nero, verde`;


/** Задание для модели по одной работе. Фото пользователь прикладывает сам. */
export function captionPrompt(title: string, authorText: string): string {
	const name = title.trim() || '(none)';
	const text = authorText.trim().replace(/\s+/g, ' ').slice(0, 600) || '(none)';
	return `ROLE
You prepare search text for a small museum website of gothic, handmade art dolls and miniatures. You write the hidden search caption of ONE piece from its photos.

WHY THIS TEXT EXISTS
The website search reads a hidden text of each piece. Visitors type short words in six languages (English, Russian, Ukrainian, Spanish, German, Italian), for example "weapon", "оружие", "animal", "Hexe", "gnomo con hacha", "red hat", "scary". The search compares WHOLE WORDS. A piece is found only if its hidden text contains the exact word the visitor typed, in the visitor's language. So the text must contain both the specific name of each visible thing (axe) AND the broader category a visitor may type instead (weapon, tool), in all six languages.

INPUT
The attached photos show one piece. Use all of them.
Title given by the author: ${name}
Description given by the author: ${text}

OUTPUT: EXACTLY TWO LINES, NOTHING ELSE
Line 1: 2 to 4 plain English sentences about what is visible (no label before it).
Line 2: "Keywords: " followed by six word lists separated by " | " in this fixed order: English | Russian | Ukrainian | Spanish | German | Italian.
No text before or after, no markdown, no code block, no headings, no explanations.

KEYWORD LISTS
- 25 to 45 items per language. All six lists contain the SAME concepts in the same order, translated the way a native speaker would type them (not word by word). A language may add local synonyms at the end of its own list.
- Put the most visible and most important items first.
- Prefer single words. A short phrase (two or three words) is allowed for colour plus item (red cap, houndstooth vest), but the search needs the bare nouns, so also give the bare noun as its own item (cap, vest).
- Separate items with ", ". No sentences, no numbering, no brackets, no semicolons, no vertical bar inside a list.
- German nouns start with a capital letter, everything else is lower case. No transliteration (never write "topor" for топор).
- No item twice inside one list.

WHAT TO LIST (go through this for every photo)
1. Character: witch, gnome, old woman, old man, monk, vampire, musician, girl, pirate ..., apparent age, and a folklore name only if the photos and the author's title clearly support it (Baba Yaga). Give BOTH the formal word and the everyday word people really type: old woman AND granny AND grandmother; old man AND grandfather; girl; boy.
2. Pose and action: holding, sitting, bending, reading, playing, looking down.
2a. Face and hair: the expression (stern, grumpy, smiling, sad, surprised, sleepy), signs of age (wrinkled), hair colour and length, beard, moustache, glasses.
3. Every held or nearby object by its plain name AND, as a separate item, its category. The only categories to use are: weapon (axe, sword, spear, dagger, bow, club, pistol), tool (broom, hammer, pickaxe, axe), light source (candle, lantern), container (basket, bag, backpack, bottle, cauldron), musical instrument (accordion, violin, flute), book, food, plant (moss, flowers, mushrooms), treasure.
4. Every animal and bird by name AND the item "animal" (birds also "bird").
5. Clothing and accessories, each by its plain everyday word, with colours and patterns. If you cannot tell two garments apart, list both words (hood AND headscarf, vest AND waistcoat, skirt AND apron). Do NOT add abstract categories such as "headwear" or "footwear": nobody types them.
6. Setting if visible: forest, night, snow, kitchen table, stone wall.
7. Mood in 3 to 6 words a visitor may search by: cozy, playful, calm, melancholic, solemn, rustic, grumpy, mysterious, gloomy, eerie. Choose only moods that the face, the pose or the light really show. A plain, evenly lit photo on a calm background is "calm" or "rustic", NOT "gloomy", "eerie" or "scary". Use "gloomy", "eerie", "scary" only for clearly dark or threatening images (dark light, grim pose, skulls, blood).
8. Dominant colours.

RULES
- Describe ONLY what is really visible. Do not guess, do not invent a story, do not add objects that are not in the photos. If unsure, leave it out.
- Use the author's title and description only to confirm a name or identity the photos support. Never add something that is not visible because the author's text mentions it.
- Do NOT list words that every piece in this museum has, because they help nobody find anything: doll, figure, figurine, miniature, small, tiny, art, handmade, hand-painted, polymer clay, acrylic, fabric, textile, gothic, toy, and their equivalents in the other five languages (кукла, лялька, фигурка, миниатюра, ручная работа, muñeca, Puppe, bambola ...).

SELF-CHECK BEFORE ANSWERING
- Are there exactly two lines, and does line 2 start with "Keywords: "?
- Are there six lists separated by " | " in the order EN, RU, UK, ES, DE, IT, with the same concepts in the same order?
- Does every held object have its category item (axe -> weapon, tool)? Does every animal have "animal" (birds also "bird")?
- Did you give the everyday words for people (granny, grandfather) and the exact garment words (headscarf, apron, vest), and the facial expression and hair?
- Is every mood word supported by the photos? Remove "gloomy", "eerie", "scary" if the photo is calm.
- Is any banned generic word, or an abstract category such as "headwear", in a list? Is any item not visible in the photos? Remove it.

EXAMPLES (they show the level of detail; do not copy their words)
Example 1. Photos: a gnome with a long white beard, red cap with white dots, houndstooth vest, beige shirt, red trousers, brown shoes, bent forward, holding an axe; soft light, rustic mood.

${EXAMPLE_GNOME}

Example 2. Photos: an old woman with grey hair under a brown woven headscarf and a stern wrinkled face, a broom made of a branch with straw, beige blouse, white fur vest, brown coarse skirt with green moss, dark shoes; calm even light, white floral wallpaper, dark table.

${EXAMPLE_GRANNY}

Now write the two lines for the piece in the attached photos.`;
}

export type CaptionNote =
	| { code: 'noKeywords' }
	| { code: 'segments'; n: number }
	| { code: 'few'; lang: string; n: number }
	| { code: 'markdown' };

export interface CaptionCheck {
	/** Текст без обрамляющих ``` и пустых краёв: его и сохраняют. */
	text: string;
	/** Число слов в списках (по всем языкам). */
	words: number;
	/** Сколько языков из шести найдено в строке Keywords. */
	languages: number;
	notes: CaptionNote[];
}

/** Слов в списке языка меньше этого — подпись для него почти пуста. */
const MIN_WORDS = 10;

/** Что прислала модель: чистит обрамление и говорит, чего не хватает. Не блокирует сохранение. */
export function checkCaption(raw: string): CaptionCheck {
	// Чат превращает перенос строки в «\» в конце строки; в подписи он не нужен.
	const text = raw
		.split('\n')
		.filter((l) => !/^\s*```/.test(l))
		.map((l) => l.replace(/\s*\\+\s*$/, ''))
		.join('\n')
		.trim();
	const notes: CaptionNote[] = [];
	if (!text) return { text, words: 0, languages: 0, notes };

	if (/\*\*|^\s*#{1,6}\s/m.test(text)) notes.push({ code: 'markdown' });

	const line = text.split('\n').find((l) => /^\s*keywords\s*:/i.test(l));
	if (!line) {
		notes.push({ code: 'noKeywords' });
		return { text, words: 0, languages: 0, notes };
	}
	const lists = line
		.replace(/^\s*keywords\s*:/i, '')
		.split('|')
		.map((s) =>
			s
				.split(',')
				.map((w) => w.trim())
				.filter(Boolean),
		)
		.filter((l) => l.length > 0);
	if (lists.length !== CAPTION_LANGS.length) notes.push({ code: 'segments', n: lists.length });
	lists.forEach((l, i) => {
		if (l.length < MIN_WORDS) notes.push({ code: 'few', lang: CAPTION_LANGS[i] ?? String(i + 1), n: l.length });
	});
	return { text, words: lists.reduce((a, l) => a + l.length, 0), languages: lists.length, notes };
}
