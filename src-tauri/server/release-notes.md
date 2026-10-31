# Заметки релизов

Файл читает сервер (`src/services/release.rs`): новейший релиз стоит первым.
Когда в нём появляется номер, о котором ещё не заводили черновик, в админке
(вкладка «Канал») ждёт пост для Telegram-каналов. Номер — любой, лишь бы он
был новый и стоял после `## `.

Формат: `## номер · дата`, затем `### язык` (en обязателен, остальные — по
готовности; без языка пост выходит на английском с пометкой) и строки
`new:` (новое) и `better:` (стало лучше). До шести пунктов каждого рода,
до 220 знаков. Тон как у всего канала: без восклицаний и «успейте».
Новые работы, байки, карты и битвы в пункты не пишутся — сервер считает их
сам по журналу.

## 2026-10-29 · Уведомления об игре

### en
new: Notifications have a fourth topic, “Cards and battles”: a message when a new card or battle appears
better: Existing subscribers are not asked anything and are not sent old cards; the topic is theirs to switch on

### de
new: Die Benachrichtigungen haben ein viertes Thema, „Karten und Schlachten“: eine Nachricht, wenn eine neue Karte oder Schlacht erscheint
better: Bestehende Abonnenten werden nicht gefragt und bekommen keine alten Karten; das Thema schalten sie selbst ein

### es
new: Las notificaciones tienen un cuarto tema, «Cartas y batallas»: un aviso cuando aparece una carta o una batalla nueva
better: A quienes ya están suscritos no se les pregunta nada ni se les envían cartas antiguas; el tema lo activan ellos

### it
new: Le notifiche hanno un quarto tema, «Carte e battaglie»: un avviso quando compare una carta o una battaglia nuova
better: Agli iscritti non si chiede nulla e non si inviano carte vecchie; il tema lo attivano loro

### ru
new: У уведомлений появилась четвёртая тема, «Карты и битвы»: сообщение, когда выходит новая карта или битва
better: Уже подписанным ничего не навязывается и старые карты не присылаются; тему они включают сами

## 2026-10-22 · Каналы по языкам

### en
new: The site is now also in Spanish, German and Italian
new: “Ask the site” (Ctrl/⌘ K): short answers about works, tales and the game
better: The search in the header finds works, tales, the gazette and cards at once
better: The Telegram channel now has a version in every language

### de
new: Die Seite gibt es jetzt auch auf Spanisch, Deutsch und Italienisch
new: „Die Seite fragen“ (Strg/⌘ K): kurze Antworten zu Werken, Geschichten und zum Spiel
better: Die Suche oben findet Werke, Geschichten, Zeitung und Karten auf einmal
better: Den Telegram-Kanal gibt es jetzt in jeder Sprache

### es
new: El sitio ahora también está en español, alemán e italiano
new: «Pregunta al sitio» (Ctrl/⌘ K): respuestas breves sobre obras, cuentos y el juego
better: La búsqueda de arriba encuentra obras, cuentos, la gaceta y cartas a la vez
better: El canal de Telegram ahora existe en cada idioma

### it
new: Il sito ora è anche in spagnolo, tedesco e italiano
new: «Chiedi al sito» (Ctrl/⌘ K): risposte brevi su opere, racconti e gioco
better: La ricerca in alto trova opere, racconti, gazzetta e carte in un colpo solo
better: Il canale Telegram ora esiste in ogni lingua

### ru
new: Сайт теперь есть и на испанском, немецком и итальянском
new: «Спросить сайт» (Ctrl/⌘ K): короткие ответы про работы, байки и игру
better: Поиск в шапке находит работы, байки, вестник и карты разом
better: У Telegram-канала теперь есть версия на каждом языке
