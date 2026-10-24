#!/usr/bin/env python3
"""Вторая партия каталога «Спросить сайт»: новые ответы и переводы старых.

Что делает миграция:
  1. дописывает испанский, немецкий и итальянский к вопросу, ответу и подписям
     ссылок у записей первой партии (только пустые места: правку из админки не
     затирает; ссылки меняет, только если они ещё такие, как были заведены);
  2. заводит новые записи на шести языках (`ON CONFLICT DO NOTHING`);
  3. включает «Как получить новые карты?»: сайт сам говорит, что карты покупают
     за пыль, а пыль начисляется за просмотры и чтение.

Все тексты взяты из того, что сайт уже говорит (страницы «О фотографиях»,
«Условия использования», «Как приобрести работу», подписи интерфейса, окно
«Спросить сайт»). Чего сайт не говорит, заведено выключенным черновиком без
ответа: ответ пишет автор в админке. Слова интерфейса на каждом языке взяты из
`src/lib/i18n/*.ts`, чтобы ответ называл кнопку так, как она подписана.

Применённые миграции не правят (sqlx сверяет контрольную сумму). Запуск:
    python3 tools/ask/make_more_migration.py [ИМЯ_ФАЙЛА.sql]
"""
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
OUT_DIR = HERE.parent.parent / "src-tauri" / "server" / "migrations"
FIRST = OUT_DIR / "20261022000004_ask_answers.sql"
LANGS = ["en", "ru", "uk", "es", "de", "it"]


def T(en, ru, uk, es, de, it):
    return dict(zip(LANGS, [en, ru, uk, es, de, it]))


def tail(es, de, it):
    return {"es": es, "de": de, "it": it}


# ── Подписи ссылок ───────────────────────────────────────────────────────────
ACQ = T("How to acquire a work", "Как приобрести работу", "Як придбати роботу", "Cómo adquirir una obra", "Wie man ein Werk erhält", "Come avere un’opera")
COM = T("Commission a work", "Заказать работу", "Замовити роботу", "Encargar una obra", "Werk beauftragen", "Commissiona un’opera")
ALL = T("All works", "Все работы", "Усі роботи", "Todas las obras", "Alle Werke", "Tutte le opere")
MY = T("My requests", "Мои обращения", "Мої звернення", "Mis solicitudes", "Meine Anfragen", "Le mie richieste")
AUTHOR = T("About the author", "Об авторе", "Про автора", "Sobre la autora", "Über die Autorin", "L’autrice")
WORKSHOP = T("The workshop", "Мастерская", "Майстерня", "El taller", "Die Werkstatt", "Il laboratorio")
STUDIES = T("Studies", "Этюды", "Етюди", "Desafíos", "Etüden", "Étude")
SHELF = T("Card shelf", "Полка карт", "Полиця карт", "Estante de cartas", "Kartenregal", "Scaffale delle carte")
TALES = T("Tall tales", "Небылицы", "Небилиці", "Cuentos", "Geschichten", "Racconti")
GAZETTE = T("Gazette", "Вестник", "Вісник", "Gaceta", "Zeitung", "Gazzetta")
PRIVACY = T("Privacy", "Конфиденциальность", "Конфіденційність", "Privacidad", "Datenschutz", "Privacy")
LOGIN = T("Sign in", "Войти", "Увійти", "Entrar", "Anmelden", "Entra")
REGISTER = T("Create a name", "Завести имя", "Завести ім’я", "Crear cuenta", "Konto erstellen", "Crea l’account")
WRITE = T("Write to the master", "Написать мастеру", "Написати майстру", "Escribir a la autora", "Der Autorin schreiben", "Scrivi all’autrice")
PHOTOS = T("The photographs", "О фотографиях", "Про фотографії", "Las fotografías", "Die Fotos", "Le fotografie")
BOOKINGS = T("My Reservations", "Мои брони", "Мої броні", "Mis reservas", "Meine Reservierungen", "Le mie prenotazioni")
TERMS = T("Terms of Use", "Условия использования", "Умови користування", "Condiciones de uso", "Nutzungsbedingungen", "Condizioni d’uso")
WISHLIST = T("Wishlist", "Избранное", "Обране", "Guardadas", "Merkliste", "Salvate")
STUDIO = T("Studio", "Студия", "Студія", "Estudio", "Studio", "Studio")

# Подписи ссылок первой партии по адресу (en, ru, uk там уже есть).
LABEL_TAIL = {
    "/acquire": ACQ, "/commission": COM, "/figurines": ALL, "/profile/dealings": MY,
    "/author": AUTHOR, "/workshop": WORKSHOP, "/battles/etude": STUDIES, "/battles": SHELF,
    "/tales": TALES, "/gazette": GAZETTE, "/privacy": PRIVACY, "/login": LOGIN, "/register": REGISTER,
}
# Исключения: тот же адрес, другая подпись у записи.
LABEL_TAIL_BY_KEY = {("bot_nature", "/author"): WRITE}

# ── Переводы записей первой партии: key -> (вопрос, ответ) на es/de/it ───────
FILL = {
    "lead_time": (
        tail("¿Cuánto tiempo lleva hacer una obra?", "Wie lange dauert die Anfertigung eines Werks?", "Quanto tempo ci vuole per realizzare un’opera?"), None),
    "care": (
        tail("¿Cómo cuido una figura?", "Wie pflege ich eine Figur?", "Come si cura una statuina?"), None),
    "discounts": (
        tail("¿Hay descuentos u ofertas?", "Gibt es Rabatte oder Aktionen?", "Ci sono sconti o promozioni?"), None),
    "gift": (
        tail("Quiero elegir una obra como regalo", "Ich möchte ein Werk als Geschenk aussuchen", "Vorrei scegliere un’opera come regalo"), None),
    "how_order": (
        tail("¿Cómo encargo o adquiero una obra?", "Wie bestelle oder erwerbe ich ein Werk?", "Come ordino o ottengo un’opera?"),
        tail(
            "Elige una obra terminada y envía una solicitud, o encarga una nueva. Escribes directamente a la autora; no se cobra ni se debe nada hasta que las dos partes estén de acuerdo. Si la autora acepta el trabajo, te escribe en persona con las condiciones.",
            "Wähle ein fertiges Werk und sende eine Anfrage oder gib ein neues in Auftrag. Du schreibst der Künstlerin direkt; nichts wird berechnet und nichts ist geschuldet, bis ihr euch einig seid. Nimmt die Künstlerin das Werk an, schreibt sie dir persönlich die Bedingungen.",
            "Scegli un’opera finita e invia una richiesta, oppure commissionane una nuova. Scrivi direttamente all’autrice; non si paga niente e non si deve niente finché non siete d’accordo. Se l’autrice accetta, ti scrive di persona con le condizioni.",
        )),
    "price": (
        tail("¿Cuánto cuesta una obra?", "Was kostet ein Werk?", "Quanto costa un’opera?"),
        tail(
            "En el sitio no hay precios: la autora indica las condiciones personalmente, por mensaje. Hasta que estéis de acuerdo, no se cobra nada.",
            "Auf der Seite gibt es keine Preise: Die Bedingungen nennt die Autorin persönlich, per Nachricht. Bis ihr euch einig seid, wird nichts berechnet.",
            "Sul sito non ci sono prezzi: le condizioni le indica l’autrice di persona, per messaggio. Finché non siete d’accordo, non si paga nulla.",
        )),
    "shipping": (
        tail("¿Hacen envíos y a qué países?", "Gibt es Versand, und in welche Länder?", "C’è la spedizione e in quali paesi?"),
        tail(
            "El envío está asegurado, con seguimiento y es solo de ida: la obra se embala a mano y se fotografía antes de salir. El resto se paga antes de que salga del taller.",
            "Der Versand ist versichert und nachverfolgbar, ohne Rücksendung — von Hand verpackt und vor der Reise fotografiert. Der Rest wird beglichen, bevor das Werk die Werkstatt verlässt.",
            "La spedizione è assicurata, tracciata e solo andata: l’opera è imballata a mano e fotografata prima di partire. Il saldo si paga prima che lasci il laboratorio.",
        )),
    "payment": (
        tail("¿Cómo puedo pagar el pedido?", "Wie kann ich die Bestellung bezahlen?", "Come posso pagare l’ordine?"),
        tail(
            "Una señal no reembolsable aparta la pieza y cubre materiales y tiempo. El resto se paga antes de que la obra salga del taller. Las condiciones las indica la autora personalmente.",
            "Eine nicht erstattbare Anzahlung sichert das Stück und deckt Material und Zeit. Der Rest wird beglichen, bevor das Werk die Werkstatt verlässt. Die Bedingungen nennt die Autorin persönlich.",
            "Un acconto non rimborsabile riserva l’opera e copre materiali e tempo. Il saldo si paga prima che l’opera lasci il laboratorio. Le condizioni le indica l’autrice di persona.",
        )),
    "commission": (
        tail("¿Puedo encargar una figura a medida con mi idea?", "Kann ich eine Figur nach meiner Idee in Auftrag geben?", "Posso commissionare una statuina con la mia idea?"),
        tail(
            "Sí. Envía una solicitud de obra nueva y escribe directamente a la autora: no se cobra nada hasta que estéis de acuerdo. En un encargo, la figura se hace a mano, etapa por etapa, y puedes seguir el proceso.",
            "Ja. Sende eine Anfrage für ein neues Werk und schreib der Autorin direkt: Nichts wird berechnet, bis ihr euch einig seid. Bei einem Auftrag entsteht die Figur von Hand, Schritt für Schritt, und du kannst die Arbeit verfolgen.",
            "Sì. Invia una richiesta per una nuova opera e scrivi direttamente all’autrice: non si paga niente finché non siete d’accordo. In una commissione la figura prende forma a mano, una fase alla volta, e puoi seguire i progressi.",
        )),
    "returns": (
        tail("¿Se puede devolver una obra?", "Kann ich ein Werk zurückgeben?", "Si può restituire un’opera?"),
        tail(
            "Cada pieza es frágil y única, por eso todas las ventas son definitivas y no se admiten devoluciones. El envío está asegurado y la obra se fotografía antes de salir.",
            "Jedes Stück ist zerbrechlich und einzigartig, deshalb sind alle Verkäufe endgültig und eine Rückgabe ist nicht möglich. Der Versand ist versichert, und das Werk wird vor der Reise fotografiert.",
            "Ogni pezzo è fragile e unico, per questo tutte le vendite sono definitive e non si accettano resi. La spedizione è assicurata e l’opera è fotografata prima di partire.",
        )),
    "order_status": (
        tail("¿Cómo sé en qué fase está mi pedido?", "Wie erfahre ich, wie weit meine Bestellung ist?", "Come so a che punto è il mio ordine?"),
        tail(
            "El envío tiene seguimiento. En un encargo puedes seguir el proceso y hablar con la autora por el camino. Tus solicitudes y las respuestas están en tu perfil.",
            "Der Versand ist nachverfolgbar. Bei einem Auftrag kannst du die Arbeit verfolgen und unterwegs mit der Autorin sprechen. Deine Anfragen und die Antworten liegen in deinem Profil.",
            "La spedizione è tracciata. Per una commissione puoi seguire i progressi e parlare con l’autrice lungo il percorso. Le tue richieste e le risposte sono nel tuo profilo.",
        )),
    "contact": (
        tail("¿Cómo contacto con la autora?", "Wie erreiche ich die Autorin?", "Come contatto l’autrice?"),
        tail(
            "Puedes escribir a la autora con el botón «Escribir» de la cabecera o con el formulario de la página principal. La autora lee cada mensaje y responde personalmente, normalmente en un día.",
            "Du erreichst die Autorin über die Schaltfläche „Schreiben“ in der Kopfzeile oder das Formular auf der Startseite. Die Autorin liest jede Nachricht und antwortet persönlich, meist innerhalb eines Tages.",
            "Puoi scrivere all’autrice con il pulsante «Scrivi» nell’intestazione o con il modulo nella pagina principale. L’autrice legge ogni messaggio e risponde di persona, di solito entro un giorno.",
        )),
    "author": (
        tail("¿Quién es la autora y quién hace las figuras?", "Wer ist die Autorin, und wer macht die Figuren?", "Chi è l’autrice e chi fa le statuine?"),
        tail(
            "Todas las figuras las hace una sola autora: cada una la modela, la pinta y la viste a mano, por eso no hay dos iguales.",
            "Alle Figuren macht eine einzige Autorin: Jede modelliert, bemalt und kleidet sie von Hand, deshalb gleicht keine der anderen.",
            "Tutte le statuine sono opera di una sola autrice: ognuna è modellata, dipinta e vestita a mano, per questo non ce ne sono due uguali.",
        )),
    "materials": (
        tail("¿De qué están hechas las figuras?", "Woraus sind die Figuren gemacht?", "Di che cosa sono fatte le statuine?"),
        tail(
            "Las figuras se modelan, se pintan y se visten a mano, y cada una existe en un único ejemplar. El material y la técnica de cada obra figuran en su página.",
            "Die Figuren werden von Hand modelliert, bemalt und gekleidet, und jede gibt es nur ein einziges Mal. Material und Technik jedes Werks stehen auf seiner Seite.",
            "Le statuine sono modellate, dipinte e vestite a mano, e ognuna esiste in un solo esemplare. Materiale e tecnica di ogni opera sono scritti sulla sua pagina.",
        )),
    "size": (
        tail("¿Qué tamaño tienen las figuras?", "Wie groß sind die Figuren?", "Quanto sono grandi le statuine?"),
        tail(
            "El tamaño exacto figura en la página de cada obra.",
            "Die genaue Größe steht auf der Seite jedes Werks.",
            "Le dimensioni esatte sono sulla pagina di ogni opera.",
        )),
    "unique": (
        tail("¿Cada obra es única?", "Ist jedes Werk ein Unikat?", "Ogni opera è unica?"),
        tail(
            "Cada obra es única y existe en un solo ejemplar.",
            "Jedes Werk ist einzigartig und existiert nur einmal.",
            "Ogni opera è unica ed esiste in un solo esemplare.",
        )),
    "is_shop": (
        tail("¿Esto es una tienda online?", "Ist das ein Online-Shop?", "È un negozio online?"),
        tail(
            "Es un escaparate de miniaturas góticas de autora, no una tienda. No hay carrito ni pago: para adquirir una obra, escribes a la autora.",
            "Dies ist eine Vitrine eigener gotischer Miniaturen, kein Laden. Es gibt keinen Warenkorb und keine Kasse: Um ein Werk zu erhalten, schreibst du der Autorin.",
            "È una vetrina di miniature gotiche d’autrice, non un negozio. Non ci sono carrello né cassa: per avere un’opera scrivi all’autrice.",
        )),
    "reserve": (
        tail("¿Puedo reservar una obra?", "Kann ich ein Werk reservieren?", "Posso prenotare un’opera?"),
        tail(
            "Si la autora acepta el trabajo, te escribe en persona con las condiciones. Una señal no reembolsable aparta la pieza.",
            "Nimmt die Autorin das Werk an, schreibt sie dir persönlich die Bedingungen. Eine nicht erstattbare Anzahlung sichert das Stück.",
            "Se l’autrice accetta, ti scrive di persona con le condizioni. Un acconto non rimborsabile riserva l’opera.",
        )),
    "play_rules": (
        tail("¿Cómo se juega? ¿Cuáles son las reglas?", "Wie spielt man? Welche Regeln gibt es?", "Come si gioca? Quali sono le regole?"),
        tail(
            "Un desafío es una partida contra el ordenador en un campo pequeño. Colocas tus cartas y se juega por turnos; gana quien conserva cartas cuando al rival no le queda ninguna.\n\nCada desafío puede tener sus propias reglas: están escritas en la hoja de combate antes de empezar. Los primeros combates se pueden jugar sin entrar.",
            "Eine Etüde ist eine Partie gegen den Computer auf einem kleinen Feld. Du stellst deine Karten auf, und ihr zieht abwechselnd; es gewinnt, wer noch Karten hat, wenn der Gegner keine mehr hat.\n\nJede Etüde kann eigene Regeln haben: Sie stehen vor dem Start auf dem Kampfblatt. Die ersten Kämpfe kannst du ohne Anmeldung spielen.",
            "Un’étude è una partita contro il computer su un piccolo campo. Disponi le tue carte e si gioca a turno; vince chi ha ancora carte quando l’avversario non ne ha più.\n\nOgni étude può avere regole proprie: sono scritte sul foglio della battaglia prima dell’inizio. Le prime battaglie si possono giocare senza entrare.",
        )),
    "play_first": (
        tail("¿Cómo empiezo mi primer combate?", "Wie starte ich meinen ersten Kampf?", "Come comincio la mia prima battaglia?"),
        tail(
            "Abre los desafíos y elige el primer combate. Los primeros combates se pueden jugar sin entrar, y antes de cada uno la hoja de combate cuenta sus reglas.",
            "Öffne die Etüden und wähle den ersten Kampf. Die ersten Kämpfe kannst du ohne Anmeldung spielen, und vor jedem stehen seine Regeln auf dem Kampfblatt.",
            "Apri le étude e scegli la prima battaglia. Le prime battaglie si possono giocare senza entrare, e prima di ciascuna il foglio della battaglia ne dice le regole.",
        )),
    "play_win": (
        tail("¿Cómo se gana un combate?", "Wie gewinne ich einen Kampf?", "Come si vince una battaglia?"),
        tail(
            "Ganas cuando te quedan cartas y al rival no le queda ninguna. Algunos desafíos añaden reglas propias; están escritas en la hoja de combate antes de empezar.",
            "Du gewinnst, wenn du noch Karten hast und der Gegner keine mehr. Manche Etüden haben eigene Regeln; sie stehen vor dem Start auf dem Kampfblatt.",
            "Vinci quando hai ancora carte e l’avversario non ne ha più. Alcune étude aggiungono regole proprie; sono scritte sul foglio della battaglia prima dell’inizio.",
        )),
    "play_races": (
        tail("¿Qué raza es más fuerte contra cuál?", "Welches Volk ist gegen welches stark?", "Quale razza è più forte contro quale?"),
        tail(
            "Cada raza golpea más fuerte a algunas razas y está protegida frente a otras: +1 de daño en golpes y hechizos contra las razas señaladas, y 1 menos de daño recibido de otras. Está escrito en la hoja de cada carta, en la sección «Raza en batalla».",
            "Jedes Volk trifft einige Völker härter und ist vor anderen geschützt: +1 Schaden mit Schlägen und Zaubern gegen die markierten Völker und 1 Schaden weniger von anderen. Es steht auf dem Blatt jeder Karte im Abschnitt „Volk im Kampf“.",
            "Ogni razza colpisce più forte alcune razze ed è protetta da altre: +1 danno con colpi e incantesimi contro le razze indicate, e 1 danno in meno da altre. È scritto sul foglio di ogni carta, nella sezione «Razza in battaglia».",
        )),
    "play_cards_get": (
        tail("¿Cómo consigo cartas nuevas?", "Wie bekomme ich neue Karten?", "Come ottengo nuove carte?"), None),
    "tale_what": (
        tail("¿Qué es un cuento?", "Was ist eine Geschichte?", "Che cos’è un racconto?"),
        tail(
            "Un cuento es una historia breve sobre una figura. Muchos tienen su propia obra, y la obra tiene una carta en el juego.",
            "Eine Geschichte ist ein kurzer Text über eine Figur. Viele haben ein eigenes Werk, und das Werk hat eine Karte im Spiel.",
            "Un racconto è una breve storia su una statuina. Molti hanno una propria opera, e l’opera ha una carta nel gioco.",
        )),
    "gazette_what": (
        tail("¿Qué es la Gaceta?", "Was ist die Zeitung?", "Che cos’è la Gazzetta?"),
        tail(
            "La Gaceta es una serie de hojas breves: lo que ha llegado al taller y noticias sobre miniaturas. Los cuentos también son hojas, pero tienen su propio estante.",
            "Die Zeitung ist eine Folge kurzer Blätter: was in der Werkstatt neu ist und Neuigkeiten rund um Miniaturen. Geschichten sind auch Blätter, haben aber ihr eigenes Regal.",
            "La Gazzetta è una serie di brevi fogli: cosa è arrivato nel laboratorio e notizie sulle miniature. Anche i racconti sono fogli, ma hanno il loro scaffale.",
        )),
    "unsubscribe": (
        tail("¿Cómo me doy de baja del boletín?", "Wie melde ich mich vom Newsletter ab?", "Come annullo l’iscrizione alla newsletter?"),
        tail(
            "Cada correo del sitio lleva un enlace para darse de baja. En el bot de Telegram, escribe /stop.",
            "Jeder Brief der Seite enthält einen Abmeldelink. Im Telegram-Bot schreib /stop.",
            "Ogni lettera del sito contiene un link per annullare l’iscrizione. Nel bot di Telegram scrivi /stop.",
        )),
    "privacy": (
        tail("¿Qué datos míos guardáis?", "Welche Daten von mir speichert ihr?", "Quali miei dati conservate?"),
        tail(
            "Lo que el sitio guarda sobre los visitantes y durante cuánto tiempo está escrito en la página de privacidad.",
            "Was die Seite über Besucher speichert und wie lange, steht auf der Datenschutzseite.",
            "Che cosa il sito conserva sui visitatori e per quanto tempo è scritto nella pagina sulla privacy.",
        )),
    "account": (
        tail("¿Hace falta registrarse?", "Muss ich mich registrieren?", "Serve registrarsi?"),
        tail(
            "Puedes ver el sitio y jugar los primeros combates sin entrar. Para entrar, usa tu correo con cuatro símbolos, Telegram o Google.",
            "Du kannst die Seite ansehen und die ersten Kämpfe ohne Anmeldung spielen. Anmelden kannst du dich mit E-Mail und vier Symbolen, über Telegram oder Google.",
            "Puoi guardare il sito e giocare le prime battaglie senza entrare. Per entrare usa l’email con quattro simboli, Telegram o Google.",
        )),
    "forgot_signs": (
        tail("He olvidado mis signos de acceso", "Ich habe meine Zeichen zum Anmelden vergessen", "Ho dimenticato i simboli di accesso"),
        tail(
            "En la página de entrada elige «¿Olvidaste tus signos?». El enlace para elegir signos nuevos llega por cada vía de vuelta confirmada: a tu correo confirmado y a Telegram, si está vinculado. El enlace vale dos horas.",
            "Wähle auf der Anmeldeseite „Zeichen vergessen?“. Der Link zum Wählen neuer Zeichen kommt auf jedem bestätigten Rückweg an: an deine bestätigte E-Mail und an Telegram, falls verknüpft. Der Link gilt zwei Stunden.",
            "Nella pagina di accesso scegli «Hai dimenticato i simboli?». Il link per scegliere simboli nuovi arriva su ogni via di ritorno confermata: alla tua email confermata e a Telegram, se è collegato. Il link vale due ore.",
        )),
    "bot_nature": (
        tail("¿Eres una red neuronal o una persona?", "Bist du ein neuronales Netz oder ein Mensch?", "Sei una rete neurale o una persona?"),
        tail(
            "No es una persona ni una red neuronal: esta ventana elige la respuesta por el sentido de tu pregunta entre respuestas ya hechas y los datos del propio sitio.",
            "Weder ein Mensch noch ein neuronales Netz: Dieses Fenster wählt die Antwort nach dem Sinn deiner Frage aus fertigen Antworten und den Daten der Seite selbst.",
            "Né una persona né una rete neurale: questa finestra sceglie la risposta in base al senso della tua domanda tra risposte pronte e i dati del sito stesso.",
        )),
    "data_count": (tail("¿Cuántas obras hay en el sitio?", "Wie viele Werke gibt es auf der Seite?", "Quante opere ci sono sul sito?"), None),
    "data_new": (tail("¿Qué hay de nuevo?", "Was gibt es Neues?", "Che novità ci sono?"), None),
    "data_read": (tail("¿Qué puedo leer en cinco minutos?", "Was kann ich in fünf Minuten lesen?", "Cosa posso leggere in cinque minuti?"), None),
    "data_start": (tail("¿Por dónde empiezo a conocer el sitio?", "Womit fange ich an, die Seite kennenzulernen?", "Da dove comincio a conoscere il sito?"), None),
}

# ── Новые записи ─────────────────────────────────────────────────────────────
# key, topic, pages, sort_order, enabled, phrases, question, answer, links
NEW = []


def add(key, topic, pages, order, phrases, question, answer, links=(), enabled=True):
    NEW.append(dict(key=key, topic=topic, pages=pages, order=order, enabled=enabled,
                    phrases=phrases, question=question, answer=answer, links=list(links)))


add("photo_share", "rights", [], 400,
    ["Can I post your photographs on Instagram or Pinterest?", "Am I allowed to share the pictures?",
     "Можно ли репостить фотографии работ?", "Разрешено ли делиться снимками в Pinterest и Instagram?", "Можно ли сохранить фото в Pinterest?",
     "Чи можна ділитися фотографіями робіт?",
     "¿Se pueden publicar las fotos en Instagram o Pinterest?", "¿Puedo guardar una foto en Pinterest?",
     "Darf ich die Fotos auf Instagram oder Pinterest posten?", "Kann ich Bilder von der Seite teilen?",
     "Posso pubblicare le foto su Instagram o Pinterest?", "Si possono condividere le immagini?"],
    T("Can I share your photos on social media?", "Можно ли выложить ваши фото в соцсети?", "Чи можна поділитися вашими фото в соцмережах?",
      "¿Puedo compartir vuestras fotos en redes sociales?", "Darf ich eure Fotos in sozialen Netzwerken teilen?", "Posso condividere le vostre foto sui social?"),
    T("Yes. You can pin an image from this site, share a page, or repost from the author’s own accounts, as long as the credit stays in place — that is showing the work, not taking it.\n\nYou may not take a photograph to sell it, print it, use it as your own product shot or present the work as your own. Do not remove the caption or the author’s name, and do not use the photographs to train an image generator.",
      "Да. Можно закрепить снимок с этого сайта в Pinterest, поделиться страницей или сделать репост с аккаунтов автора, не снимая имени, — это значит показать работу, а не присвоить её.\n\nНельзя брать фотографию, чтобы продать её, напечатать, поставить как снимок своего товара или выдать работу за свою. Нельзя убирать подпись и имя автора и нельзя отдавать снимки генератору изображений.",
      "Так. Можна закріпити знімок із цього сайту в Pinterest, поділитися сторінкою або зробити репост з акаунтів автора, не знімаючи імені, — це означає показати роботу, а не привласнити її.\n\nНе можна брати фотографію, щоб продати її, надрукувати, поставити як знімок власного товару або видати роботу за свою. Не можна прибирати підпис і ім’я автора й не можна віддавати знімки генератору зображень.",
      "Sí. Puedes fijar una imagen de este sitio en Pinterest, compartir una página o republicar desde las cuentas de la autora, dejando el nombre en su sitio: eso es mostrar la obra, no apropiársela.\n\nNo se puede tomar una fotografía para venderla, imprimirla, usarla como foto de tu producto ni presentar la obra como tuya. No quites la firma ni el nombre de la autora, y no uses las fotos para entrenar un generador de imágenes.",
      "Ja. Du kannst ein Bild von dieser Seite auf Pinterest anheften, eine Seite teilen oder von den eigenen Konten der Autorin weiterposten, solange der Name stehen bleibt — das heißt, das Werk zu zeigen, nicht, es sich anzueignen.\n\nDu darfst ein Foto nicht nehmen, um es zu verkaufen, zu drucken, als Produktfoto zu verwenden oder das Werk als dein eigenes auszugeben. Entferne weder Bildunterschrift noch den Namen der Autorin, und nutze die Fotos nicht, um einen Bildgenerator zu trainieren.",
      "Sì. Puoi fissare un’immagine di questo sito su Pinterest, condividere una pagina o ripubblicare dagli account dell’autrice, lasciando il nome al suo posto: significa mostrare l’opera, non appropriarsene.\n\nNon si può prendere una fotografia per venderla, stamparla, usarla come foto del proprio prodotto o presentare l’opera come propria. Non togliere la didascalia né il nome dell’autrice, e non usare le foto per addestrare un generatore di immagini."),
    [("/rights", PHOTOS)])

add("photo_use", "rights", [], 410,
    ["How do I get permission to reproduce a photograph?", "Can I publish your photo in a book or article?",
     "Как получить разрешение на публикацию фотографии?", "Можно использовать снимки в статье, книге или на выставке?", "Нужна ли лицензия на фото?",
     "Як отримати дозвіл на публікацію фото?",
     "¿Cómo pido permiso para reproducir una fotografía?", "¿Puedo publicar vuestras fotos en un libro o artículo?",
     "Wie bekomme ich die Erlaubnis, ein Foto zu veröffentlichen?", "Darf ich die Bilder in einem Buch oder Artikel nutzen?",
     "Come ottengo il permesso di riprodurre una fotografia?", "Posso pubblicare le foto in un libro o in un articolo?"],
    T("Can I use a photo in a magazine or an exhibition?", "Можно ли использовать фото в журнале или на выставке?", "Чи можна використати фото в журналі або на виставці?",
      "¿Puedo usar una foto en una revista o en una exposición?", "Darf ich ein Foto in einer Zeitschrift oder Ausstellung verwenden?", "Posso usare una foto in una rivista o in una mostra?"),
    T("Write first. Say which image, where it will appear and for how long. A written reply granting use is the licence; until then there is none, and silence is not permission.\n\nThere is no checkout and no standard fee: each request is considered by the author personally.",
      "Сначала напишите. Укажите, какой снимок, где он появится и на какой срок. Лицензией будет письменный ответ с разрешением; до него лицензии нет, а молчание — не разрешение.\n\nКассы и прайса нет: каждую просьбу автор рассматривает лично.",
      "Спершу напишіть. Вкажіть, який знімок, де він з’явиться і на який термін. Ліцензією буде письмова відповідь із дозволом; до неї ліцензії немає, а мовчання — не дозвіл.\n\nКаси й прайсу немає: кожне прохання автор розглядає особисто.",
      "Escribe primero. Indica qué imagen, dónde aparecerá y por cuánto tiempo. La licencia es una respuesta escrita que concede el uso; hasta entonces no hay ninguna, y el silencio no es permiso.\n\nNo hay caja ni tarifa fija: la autora considera cada solicitud personalmente.",
      "Schreib zuerst. Nenne, welches Bild, wo es erscheinen soll und für wie lange. Die Lizenz ist eine schriftliche Antwort, die die Nutzung erlaubt; bis dahin gibt es keine, und Schweigen ist keine Erlaubnis.\n\nEs gibt keine Kasse und keinen Standardpreis: Die Autorin prüft jede Anfrage persönlich.",
      "Scrivi prima. Indica quale immagine, dove comparirà e per quanto tempo. La licenza è una risposta scritta che concede l’uso; fino ad allora non ce n’è nessuna, e il silenzio non è un permesso.\n\nNon c’è cassa né tariffa fissa: l’autrice valuta ogni richiesta di persona."),
    [("/rights", PHOTOS)])

add("booking_not_purchase", "order", ["work"], 420,
    ["Does booking a work mean I bought it?", "Is my request binding?",
     "Если я забронировал работу, я её купил?", "Обязывает ли заявка к покупке?", "Заявка — это договор?",
     "Чи зобов’язує заявка до купівлі?",
     "¿Reservar una obra significa que ya la he comprado?", "¿La solicitud me obliga a algo?",
     "Habe ich das Werk gekauft, wenn ich reserviere?", "Ist meine Anfrage verbindlich?",
     "Se prenoto un’opera, l’ho comprata?", "La richiesta mi vincola?"],
    T("Is a booking the same as a purchase?", "Бронь — это покупка?", "Бронь — це купівля?",
      "¿Una reserva es lo mismo que una compra?", "Ist eine Reservierung dasselbe wie ein Kauf?", "Una prenotazione equivale a un acquisto?"),
    T("No. A booking, order, reservation, waitlist entry or question sent through the site is a request, not a contract. Nothing is sold until the author has replied personally and you have agreed on the work, the price and the terms.\n\nThe author may decline or cancel a request, and you can cancel yours.",
      "Нет. Бронь, заказ, резерв, запись в лист ожидания или вопрос, отправленные через сайт, — это заявка, а не договор. Ничего не продано, пока автор лично не ответит и вы не договоритесь о работе, цене и условиях.\n\nАвтор вправе отказать или отменить заявку, а вы можете отменить свою.",
      "Ні. Бронь, замовлення, резерв, запис до листа очікування чи запитання, надіслані через сайт, — це заявка, а не договір. Нічого не продано, доки автор особисто не відповість і ви не домовитеся про роботу, ціну й умови.\n\nАвтор має право відмовити або скасувати заявку, а ви можете скасувати свою.",
      "No. Una reserva, un pedido, una lista de espera o una pregunta enviados por el sitio son una solicitud, no un contrato. No se vende nada hasta que la autora responda personalmente y acordéis la obra, el precio y las condiciones.\n\nLa autora puede rechazar o cancelar una solicitud, y tú puedes cancelar la tuya.",
      "Nein. Eine Reservierung, Bestellung, Warteliste oder Frage über die Seite ist eine Anfrage, kein Vertrag. Nichts ist verkauft, bis die Autorin persönlich geantwortet hat und ihr euch über Werk, Preis und Bedingungen einig seid.\n\nDie Autorin kann eine Anfrage ablehnen oder stornieren, und du kannst deine stornieren.",
      "No. Una prenotazione, un ordine, una riserva, l’iscrizione alla lista d’attesa o una domanda inviati dal sito sono una richiesta, non un contratto. Non si vende nulla finché l’autrice non ha risposto di persona e non vi siete accordati su opera, prezzo e condizioni.\n\nL’autrice può rifiutare o annullare una richiesta, e tu puoi annullare la tua."),
    [("/acquire", ACQ)])

add("cancel_booking", "order", [], 430,
    ["I want to cancel my reservation", "Where is my cancel link?",
     "Хочу отменить бронь", "Где ссылка для отмены брони?", "Как отказаться от заявки?",
     "Хочу скасувати бронь",
     "Quiero cancelar mi reserva", "¿Dónde está el enlace para cancelar?",
     "Ich möchte meine Reservierung stornieren", "Wo finde ich den Stornierungslink?",
     "Voglio annullare la mia prenotazione", "Dov’è il link per annullare?"],
    T("How do I cancel a booking?", "Как отменить бронь?", "Як скасувати бронь?",
      "¿Cómo cancelo una reserva?", "Wie storniere ich eine Reservierung?", "Come annullo una prenotazione?"),
    T("Open the link you received when you booked — it carries your cancel code — and press “Cancel reservation”. Your reservations are also listed on the “My Reservations” page (the link is in the footer) and in your profile if you were signed in.\n\nA reservation the author has already confirmed cannot be cancelled with the button: write to the author.",
      "Откройте ссылку, которую вы получили при брони, — в ней лежит код отмены, — и нажмите «Отменить бронь». Ваши брони есть и на странице «Мои брони» (ссылка в подвале сайта), а если вы входили в аккаунт, то и в профиле.\n\nБронь, которую автор уже подтвердил, кнопкой не отменить: напишите автору.",
      "Відкрийте посилання, яке ви отримали під час броні, — у ньому лежить код скасування, — і натисніть «Скасувати бронь». Ваші броні є й на сторінці «Мої броні» (посилання у підвалі сайту), а якщо ви входили в акаунт, то й у профілі.\n\nБронь, яку автор уже підтвердив, кнопкою не скасувати: напишіть авторові.",
      "Abre el enlace que recibiste al reservar —lleva tu código de cancelación— y pulsa «Cancelar la reserva». Tus reservas también aparecen en la página «Mis reservas» (el enlace está al pie del sitio) y, si habías entrado, en tu perfil.\n\nUna reserva que la autora ya ha confirmado no se cancela con el botón: escribe a la autora.",
      "Öffne den Link, den du bei der Reservierung erhalten hast — er enthält deinen Stornierungscode — und klicke auf „Reservierung stornieren“. Deine Reservierungen stehen auch auf der Seite „Meine Reservierungen“ (der Link ist im Seitenfuß) und, wenn du angemeldet warst, in deinem Profil.\n\nEine Reservierung, die die Autorin schon bestätigt hat, lässt sich nicht per Knopf stornieren: Schreib der Autorin.",
      "Apri il link che hai ricevuto al momento della prenotazione — contiene il tuo codice di annullamento — e premi «Annulla la prenotazione». Le tue prenotazioni compaiono anche nella pagina «Le mie prenotazioni» (il link è a piè di pagina) e, se avevi fatto l’accesso, nel tuo profilo.\n\nUna prenotazione già confermata dall’autrice non si annulla con il pulsante: scrivi all’autrice."),
    [("/bookings", BOOKINGS)])

add("booking_status", "order", [], 440,
    ["What do the reservation statuses mean?", "Why is my booking still pending?",
     "Что означают статусы брони?", "Почему моя бронь до сих пор не подтверждена?", "Что значит «Отклонено»?",
     "Що означають статуси броні?",
     "¿Qué significan los estados de la reserva?", "¿Por qué mi reserva sigue pendiente?",
     "Was bedeuten die Status meiner Reservierung?", "Warum ist meine Reservierung noch nicht bestätigt?",
     "Che cosa significano gli stati della prenotazione?", "Perché la mia prenotazione è ancora in attesa?"],
    T("What does “Awaiting confirmation” mean?", "Что значит «Ожидает подтверждения»?", "Що означає «Очікує підтвердження»?",
      "¿Qué significa «Pendiente de confirmación»?", "Was heißt „Wartet auf Bestätigung“?", "Che cosa significa «In attesa di conferma»?"),
    T("“Awaiting confirmation” means the author has not answered yet. “Confirmed” means the author agreed. “Declined” and “Cancelled” mean the request is closed.\n\nThe author replies personally, usually within a day. You can follow the status on the “My Reservations” page.",
      "«Ожидает подтверждения» — автор ещё не ответил. «Подтверждено» — автор согласился. «Отклонено» и «Отменено» — заявка закрыта.\n\nАвтор отвечает лично, обычно в течение дня. Следить за статусом можно на странице «Мои брони».",
      "«Очікує підтвердження» — автор ще не відповів. «Підтверджено» — автор погодився. «Відхилено» і «Скасовано» — заявку закрито.\n\nАвтор відповідає особисто, зазвичай протягом дня. Стежити за статусом можна на сторінці «Мої броні».",
      "«Pendiente de confirmación» significa que la autora aún no ha respondido. «Confirmada», que la autora ha aceptado. «Rechazada» y «Cancelada», que la solicitud está cerrada.\n\nLa autora responde personalmente, normalmente en un día. Puedes seguir el estado en la página «Mis reservas».",
      "„Wartet auf Bestätigung“ heißt, die Autorin hat noch nicht geantwortet. „Bestätigt“ heißt, die Autorin hat zugestimmt. „Abgelehnt“ und „Storniert“ heißt, die Anfrage ist abgeschlossen.\n\nDie Autorin antwortet persönlich, meist innerhalb eines Tages. Den Status siehst du auf der Seite „Meine Reservierungen“.",
      "«In attesa di conferma» significa che l’autrice non ha ancora risposto. «Confermata», che l’autrice ha accettato. «Rifiutata» e «Annullata», che la richiesta è chiusa.\n\nL’autrice risponde di persona, di solito entro un giorno. Puoi seguire lo stato nella pagina «Le mie prenotazioni»."),
    [("/bookings", BOOKINGS)])

add("waitlist", "order", ["work"], 450,
    ["Can I join a waiting list for a reserved work?", "What if someone else reserved the piece I want?",
     "Можно ли встать в очередь на зарезервированную работу?", "Что делать, если работу уже забронировали?", "Есть ли лист ожидания?",
     "Чи можна стати в чергу на зарезервовану роботу?",
     "¿Puedo apuntarme a la lista de espera de una obra reservada?", "¿Qué hago si otra persona ya la reservó?",
     "Kann ich mich für ein reserviertes Werk auf die Warteliste setzen?", "Was, wenn jemand anderes das Stück schon reserviert hat?",
     "Posso mettermi in lista d’attesa per un’opera riservata?", "Che faccio se qualcuno l’ha già prenotata?"],
    T("The work is already reserved. What can I do?", "Работа уже зарезервирована. Что делать?", "Робота вже зарезервована. Що робити?",
      "La obra ya está reservada. ¿Qué puedo hacer?", "Das Werk ist schon reserviert. Was kann ich tun?", "L’opera è già riservata. Che cosa posso fare?"),
    T("Join the waiting list on the work’s page: you will be in the queue, and we will reach out if the current reservation is released.\n\nIf you would like something of your own, you can also send a request to commission a new work.",
      "Встаньте в лист ожидания на странице работы: вы окажетесь в очереди, и мы свяжемся с вами, если текущий резерв освободится.\n\nА если хочется чего-то своего, можно отправить запрос на новую работу на заказ.",
      "Станьте в лист очікування на сторінці роботи: ви опинитеся в черзі, і ми зв’яжемося з вами, якщо поточний резерв звільниться.\n\nА якщо хочеться чогось власного, можна надіслати запит на нову роботу на замовлення.",
      "Apúntate a la lista de espera en la página de la obra: quedarás en la cola y te escribiremos si la reserva actual queda libre.\n\nSi prefieres algo propio, también puedes enviar una solicitud para encargar una obra nueva.",
      "Trag dich auf der Seite des Werks in die Warteliste ein: Du stehst dann in der Warteschlange, und wir melden uns, wenn die aktuelle Reservierung frei wird.\n\nWenn du lieber etwas Eigenes möchtest, kannst du auch eine Anfrage für ein neues Werk senden.",
      "Iscriviti alla lista d’attesa nella pagina dell’opera: sarai in coda e ti scriveremo se la riserva attuale si libera.\n\nSe preferisci qualcosa di tuo, puoi anche inviare una richiesta per commissionare una nuova opera."),
    [("/commission", COM), ("/figurines", ALL)])

add("pay_safe", "order", [], 460,
    ["Does the site take card payments?", "Will you ask for my card details?",
     "Нужно ли вводить данные карты на сайте?", "Принимает ли сайт оплату картой?", "Это безопасно — платить через сайт?",
     "Чи просить сайт дані картки?",
     "¿El sitio pide los datos de mi tarjeta?", "¿Se paga con tarjeta en la web?",
     "Verlangt die Seite meine Kartendaten?", "Kann ich auf der Seite mit Karte zahlen?",
     "Il sito chiede i dati della mia carta?", "Si paga con carta sul sito?"],
    T("Is it safe to pay on the site?", "Безопасно ли платить на сайте?", "Чи безпечно платити на сайті?",
      "¿Es seguro pagar en el sitio?", "Ist es sicher, auf der Seite zu bezahlen?", "È sicuro pagare sul sito?"),
    T("The site takes no payment and never asks for your card details. Any deposit or payment is arranged with the author directly, after you have agreed on the work and the terms.",
      "Сайт не принимает оплату и никогда не спрашивает данные карты. Любой депозит или платёж согласуется с автором напрямую, после того как вы договорились о работе и условиях.",
      "Сайт не приймає оплату й ніколи не питає дані картки. Будь-який завдаток чи платіж узгоджується з автором напряму, після того як ви домовилися про роботу й умови.",
      "El sitio no cobra nada y nunca pide los datos de tu tarjeta. Cualquier señal o pago se acuerda directamente con la autora, una vez que habéis acordado la obra y las condiciones.",
      "Die Seite nimmt keine Zahlungen an und fragt nie nach deinen Kartendaten. Jede Anzahlung oder Zahlung wird direkt mit der Autorin vereinbart, nachdem ihr euch über Werk und Bedingungen einig seid.",
      "Il sito non accetta pagamenti e non chiede mai i dati della tua carta. Qualsiasi acconto o pagamento si concorda direttamente con l’autrice, dopo che vi siete accordati su opera e condizioni."),
    [("/acquire", ACQ)])

add("save_work", "works", ["work"], 470,
    ["Where are my saved works?", "Can I make a wishlist?",
     "Где мои сохранённые работы?", "Можно ли составить избранное?", "Как добавить работу в избранное?",
     "Де мої збережені роботи?",
     "¿Dónde están mis obras guardadas?", "¿Puedo hacer una lista de favoritos?",
     "Wo finde ich meine gemerkten Werke?", "Kann ich eine Merkliste anlegen?",
     "Dove sono le mie opere salvate?", "Posso creare una lista dei preferiti?"],
    T("How do I save a work for later?", "Как сохранить работу на потом?", "Як зберегти роботу на потім?",
      "¿Cómo guardo una obra para más tarde?", "Wie merke ich mir ein Werk für später?", "Come salvo un’opera per dopo?"),
    T("Press the save button on a work’s card. Without signing in, saved works stay in this browser; after you sign in, the site offers to move them to your account, where they are listed as “Wishlist” in your profile.",
      "Нажмите кнопку «сохранить» на карточке работы. Пока вы не вошли, сохранённое лежит в этом браузере; после входа сайт предложит перенести его в аккаунт, где оно будет в профиле, в разделе «Избранное».",
      "Натисніть кнопку «зберегти» на картці роботи. Доки ви не ввійшли, збережене лежить у цьому браузері; після входу сайт запропонує перенести його в акаунт, де воно буде в профілі, у розділі «Обране».",
      "Pulsa el botón de guardar en la tarjeta de la obra. Si no has entrado, lo guardado se queda en este navegador; cuando entres, el sitio te ofrecerá pasarlo a tu cuenta, donde aparece en tu perfil como «Guardadas».",
      "Klicke auf der Karte des Werks auf die Schaltfläche zum Speichern. Ohne Anmeldung bleibt das Gemerkte in diesem Browser; nach der Anmeldung bietet die Seite an, es in dein Konto zu übernehmen, wo es in deinem Profil als „Merkliste“ steht.",
      "Premi il pulsante per salvare sulla scheda dell’opera. Senza accesso, ciò che salvi resta in questo browser; dopo l’accesso il sito propone di spostarlo nel tuo account, dove compare nel profilo come «Salvate»."),
    [("/profile/wishlist", WISHLIST)])

add("notify_new", "site", [], 480,
    ["How do I follow updates?", "Is there a newsletter or a Telegram channel?", "Can I be told when a new piece appears?",
     "Как подписаться на новости?", "Есть ли рассылка или канал в Telegram?", "Можно ли узнать, когда появится новая работа?",
     "Як підписатися на новини?",
     "¿Cómo me suscribo a las novedades?", "¿Hay boletín o canal de Telegram?",
     "Wie abonniere ich Neuigkeiten?", "Gibt es einen Newsletter oder einen Telegram-Kanal?",
     "Come mi iscrivo alle novità?", "C’è una newsletter o un canale Telegram?"],
    T("How can I find out about new works?", "Как узнавать о новых работах?", "Як дізнаватися про нові роботи?",
      "¿Cómo me entero de las obras nuevas?", "Wie erfahre ich von neuen Werken?", "Come vengo a sapere delle opere nuove?"),
    T("There are three ways. Sign the house book (the form on the site): letters from the workshop will reach you once you confirm the address. Follow the Telegram channel (the “Channel” link in the footer). Or, on a work that is still in progress, press “Tell me when it’s ready”.\n\nThe confirmation letter may land in your Spam folder.",
      "Есть три пути. Впишитесь в книгу дома (форма на сайте): письма из мастерской начнут приходить, когда вы подтвердите адрес. Подпишитесь на канал в Telegram (ссылка «Канал» в подвале). Или на работе, которая ещё в процессе, нажмите «Написать, когда будет готова».\n\nПисьмо с подтверждением может попасть в «Спам».",
      "Є три шляхи. Впишіться до книги дому (форма на сайті): листи з майстерні почнуть приходити, коли ви підтвердите адресу. Підпишіться на канал у Telegram (посилання «Канал» у підвалі). Або на роботі, яка ще в процесі, натисніть «Написати, коли буде готова».\n\nЛист із підтвердженням може потрапити до «Спаму».",
      "Hay tres vías. Firma el libro de visitas (el formulario del sitio): los correos del taller te llegarán cuando confirmes la dirección. Sigue el canal de Telegram (el enlace «Canal» al pie del sitio). O, en una obra que aún está en proceso, pulsa «Avísame cuando esté lista».\n\nEl correo de confirmación puede ir a la carpeta de spam.",
      "Es gibt drei Wege. Trag dich ins Besucherbuch ein (das Formular auf der Seite): Die Briefe der Werkstatt erreichen dich, sobald du die Adresse bestätigst. Folge dem Telegram-Kanal (der Link „Kanal“ im Seitenfuß). Oder drücke bei einem Werk, das noch in Arbeit ist, auf „Sag mir, wenn es fertig ist“.\n\nDer Bestätigungsbrief landet vielleicht im Spam.",
      "Ci sono tre strade. Firma il libro dei visitatori (il modulo sul sito): le lettere del laboratorio ti arriveranno quando confermi l’indirizzo. Segui il canale Telegram (il link «Canale» a piè di pagina). Oppure, su un’opera ancora in lavorazione, premi «Avvisami quando è pronta».\n\nLa lettera di conferma potrebbe finire nello spam."))

add("mail_missing", "site", [], 490,
    ["I did not get the confirmation email", "Where is my confirmation link?",
     "Письмо с подтверждением не приходит", "Не пришла ссылка для подтверждения почты", "Куда делось письмо?",
     "Не приходить лист із підтвердженням",
     "No recibo el correo de confirmación", "¿Dónde está el enlace de confirmación?",
     "Ich bekomme keine Bestätigungs-E-Mail", "Wo ist mein Bestätigungslink?",
     "Non ricevo l’email di conferma", "Dov’è il link di conferma?"],
    T("The confirmation letter did not arrive", "Не пришло письмо с подтверждением", "Не прийшов лист із підтвердженням",
      "No me ha llegado el correo de confirmación", "Der Bestätigungsbrief ist nicht angekommen", "La lettera di conferma non è arrivata"),
    T("Look in your Spam folder first: the confirmation letter most likely landed there. If the link is wrong or older than a week, sign the house book again — a new letter will come.",
      "Сначала загляните в папку «Спам»: письмо с подтверждением, скорее всего, лежит там. Если ссылка неверна или старше недели, впишитесь в книгу дома ещё раз — придёт новое письмо.",
      "Спершу зазирніть до теки «Спам»: лист із підтвердженням, найімовірніше, лежить там. Якщо посилання хибне або старше за тиждень, впишіться до книги дому ще раз — прийде новий лист.",
      "Mira primero en la carpeta de spam: lo más probable es que el correo de confirmación esté allí. Si el enlace no es correcto o tiene más de una semana, vuelve a firmar el libro de visitas y llegará un correo nuevo.",
      "Sieh zuerst im Spam-Ordner nach: Der Bestätigungsbrief liegt sehr wahrscheinlich dort. Ist der Link falsch oder älter als eine Woche, trag dich noch einmal ins Besucherbuch ein — ein neuer Brief kommt.",
      "Guarda prima nella cartella dello spam: la lettera di conferma con ogni probabilità è lì. Se il link è sbagliato o ha più di una settimana, firma di nuovo il libro dei visitatori — arriverà una nuova lettera."))

add("age_limit", "site", [], 500,
    ["Is there an age limit?", "Can children register?",
     "Есть ли возрастное ограничение?", "Можно ли детям регистрироваться?", "С какого возраста можно оставлять заявку?",
     "Чи є вікове обмеження?",
     "¿Hay límite de edad?", "¿Pueden registrarse los menores?",
     "Gibt es eine Altersgrenze?", "Dürfen Kinder sich registrieren?",
     "C’è un limite di età?", "I minori possono registrarsi?"],
    T("How old do I have to be to use the site?", "Сколько лет нужно, чтобы пользоваться сайтом?", "Скільки років потрібно, щоб користуватися сайтом?",
      "¿Qué edad hay que tener para usar el sitio?", "Wie alt muss man sein, um die Seite zu nutzen?", "Quanti anni bisogna avere per usare il sito?"),
    T("You must be 16 or older. Every form that collects contact details, and registration, ask you to confirm it. The confirmation is your own statement, not an identity check.",
      "Вам должно быть 16 лет. Каждая форма, собирающая контактные данные, и регистрация просят это подтвердить. Подтверждение — ваше заявление, а не проверка документов.",
      "Вам має бути 16 років. Кожна форма, що збирає контактні дані, і реєстрація просять це підтвердити. Підтвердження — ваша заява, а не перевірка документів.",
      "Tienes que tener 16 años o más. Cada formulario que recoge datos de contacto, y el registro, te piden confirmarlo. La confirmación es una declaración tuya, no una comprobación de identidad.",
      "Du musst mindestens 16 Jahre alt sein. Jedes Formular, das Kontaktdaten erfasst, und die Registrierung bitten dich, das zu bestätigen. Die Bestätigung ist deine eigene Erklärung, keine Identitätsprüfung.",
      "Devi avere almeno 16 anni. Ogni modulo che raccoglie dati di contatto, e la registrazione, ti chiedono di confermarlo. La conferma è una tua dichiarazione, non un controllo d’identità."),
    [("/terms", TERMS)])

add("delete_account", "site", [], 510,
    ["I want to delete my profile and data", "Can I close my account?",
     "Хочу удалить профиль и свои данные", "Можно ли закрыть аккаунт?", "Где удалить аккаунт?",
     "Хочу видалити профіль",
     "Quiero borrar mi perfil y mis datos", "¿Puedo cerrar mi cuenta?",
     "Ich möchte mein Profil und meine Daten löschen", "Kann ich mein Konto schließen?",
     "Voglio eliminare il mio profilo e i miei dati", "Posso chiudere il mio account?"],
    T("How do I delete my account?", "Как удалить аккаунт?", "Як видалити акаунт?",
      "¿Cómo borro mi cuenta?", "Wie lösche ich mein Konto?", "Come elimino il mio account?"),
    T("You can delete your account yourself, at any time, in your profile (the “Delete account” button in the account section). What happens to your data is described on the privacy page.",
      "Аккаунт можно удалить самому в любой момент в профиле (кнопка «Удалить аккаунт» в разделе аккаунта). Что происходит с данными, описано на странице конфиденциальности.",
      "Акаунт можна видалити самому в будь-який момент у профілі (кнопка «Видалити акаунт» у розділі акаунта). Що відбувається з даними, описано на сторінці конфіденційності.",
      "Puedes borrar la cuenta en cualquier momento desde tu perfil (el botón «Borrar la cuenta» en la sección de la cuenta). Lo que ocurre con tus datos se describe en la página de privacidad.",
      "Du kannst dein Konto jederzeit selbst in deinem Profil löschen (die Schaltfläche „Konto löschen“ im Bereich Konto). Was mit deinen Daten geschieht, steht auf der Datenschutzseite.",
      "Puoi eliminare l’account in qualsiasi momento dal tuo profilo (il pulsante «Elimina l’account» nella sezione dell’account). Che cosa succede ai tuoi dati è descritto nella pagina sulla privacy."),
    [("/privacy", PRIVACY)])

add("change_language", "site", [], 520,
    ["Is the site available in other languages?", "Which languages are available?",
     "На каких языках есть сайт?", "Как переключить язык сайта?", "Есть ли сайт на английском?",
     "Якими мовами доступний сайт?",
     "¿En qué idiomas está el sitio?", "¿Cómo pongo el sitio en inglés?",
     "In welchen Sprachen gibt es die Seite?", "Wie stelle ich die Seite auf Englisch?",
     "In quali lingue è disponibile il sito?", "Come metto il sito in inglese?"],
    T("How do I change the language?", "Как сменить язык?", "Як змінити мову?",
      "¿Cómo cambio el idioma?", "Wie ändere ich die Sprache?", "Come cambio lingua?"),
    T("The site speaks six languages: English, Russian, Ukrainian, Spanish, German and Italian. Press the language seal in the header; on a phone it is in the “More” sheet of the bottom bar. Your choice is remembered.",
      "У сайта шесть языков: английский, русский, украинский, испанский, немецкий и итальянский. Нажмите печать языка в шапке; на телефоне она в листе «Ещё» нижней панели. Ваш выбор запоминается.",
      "Сайт має шість мов: англійська, російська, українська, іспанська, німецька та італійська. Натисніть печатку мови в шапці; на телефоні вона в аркуші «Ще» нижньої панелі. Ваш вибір запам’ятовується.",
      "El sitio tiene seis idiomas: inglés, ruso, ucraniano, español, alemán e italiano. Pulsa el sello del idioma en la cabecera; en el móvil está en la hoja «Más» de la barra inferior. Tu elección se recuerda.",
      "Die Seite gibt es in sechs Sprachen: Englisch, Russisch, Ukrainisch, Spanisch, Deutsch und Italienisch. Tippe in der Kopfzeile auf das Sprachsiegel; auf dem Handy steht es im Blatt „Mehr“ der unteren Leiste. Deine Wahl wird gemerkt.",
      "Il sito ha sei lingue: inglese, russo, ucraino, spagnolo, tedesco e italiano. Premi il sigillo della lingua nell’intestazione; sul telefono si trova nel foglio «Altro» della barra in basso. La tua scelta viene ricordata."))

add("how_search", "site", [], 530,
    ["How can I find a figurine?", "What can I search for?", "Keyboard shortcut for search",
     "Как найти фигурку?", "Что можно искать в поиске?", "Горячая клавиша поиска",
     "Як знайти фігурку?",
     "¿Cómo encuentro una figura?", "¿Qué puedo buscar?",
     "Wie finde ich eine Figur?", "Wonach kann ich suchen?",
     "Come trovo una statuina?", "Che cosa posso cercare?"],
    T("How do I use the search?", "Как пользоваться поиском?", "Як користуватися пошуком?",
      "¿Cómo uso la búsqueda?", "Wie benutze ich die Suche?", "Come uso la ricerca?"),
    T("Type in the search field in the header: it looks through works, tales, the gazette and cards at once, and also takes questions. The “/” key or Ctrl+K (⌘K on a Mac) opens it from any page. Arrow keys choose a result, Enter opens it.",
      "Пишите в поле поиска в шапке: оно ищет сразу по работам, небылицам, вестнику и картам, а ещё принимает вопросы. Клавиша «/» или Ctrl+K (⌘K на Mac) открывает его с любой страницы. Стрелки выбирают результат, Enter открывает.",
      "Пишіть у полі пошуку в шапці: воно шукає одразу по роботах, небилицях, віснику й картах, а ще приймає запитання. Клавіша «/» або Ctrl+K (⌘K на Mac) відкриває його з будь-якої сторінки. Стрілки обирають результат, Enter відкриває.",
      "Escribe en el campo de búsqueda de la cabecera: busca a la vez en obras, cuentos, la Gaceta y cartas, y también acepta preguntas. La tecla «/» o Ctrl+K (⌘K en Mac) lo abre desde cualquier página. Las flechas eligen un resultado y Intro lo abre.",
      "Tippe in das Suchfeld in der Kopfzeile: Es durchsucht Werke, Geschichten, die Zeitung und Karten auf einmal und nimmt auch Fragen an. Die Taste „/“ oder Strg+K (⌘K am Mac) öffnet es von jeder Seite. Pfeiltasten wählen ein Ergebnis, Enter öffnet es.",
      "Scrivi nel campo di ricerca nell’intestazione: cerca insieme tra opere, racconti, la Gazzetta e carte, e accetta anche domande. Il tasto «/» o Ctrl+K (⌘K su Mac) lo apre da qualsiasi pagina. Le frecce scelgono un risultato, Invio lo apre."))

add("tale_listen", "stories", ["tale"], 540,
    ["Is there an audio version of the tales?", "Can the site read a story aloud?",
     "Есть ли озвучка небылиц?", "Может ли сайт прочитать историю вслух?", "Как послушать небылицу?",
     "Чи є озвучення небилиць?",
     "¿Hay versión en audio de los cuentos?", "¿El sitio puede leer una historia en voz alta?",
     "Gibt es die Geschichten als Hörversion?", "Kann die Seite eine Geschichte vorlesen?",
     "C’è una versione audio dei racconti?", "Il sito può leggere una storia ad alta voce?"],
    T("Can I listen to a tale instead of reading it?", "Можно ли послушать небылицу?", "Чи можна послухати небилицю?",
      "¿Puedo escuchar un cuento en vez de leerlo?", "Kann ich mir eine Geschichte anhören, statt sie zu lesen?", "Posso ascoltare un racconto invece di leggerlo?"),
    T("Yes, if your browser has a voice for the language of the page: then a “Listen” button stands at the tale. The voice is your browser’s own, nothing is sent anywhere. You can pause, stop and change the reading speed.",
      "Да, если у вашего браузера есть голос на языке страницы: тогда у небылицы стоит кнопка «Послушать». Голос — собственный голос браузера, ничего никуда не отправляется. Можно поставить на паузу, остановить и изменить скорость чтения.",
      "Так, якщо у вашого браузера є голос мовою сторінки: тоді біля небилиці стоїть кнопка «Послухати». Голос — власний голос браузера, нічого нікуди не надсилається. Можна поставити на паузу, зупинити й змінити швидкість читання.",
      "Sí, si tu navegador tiene una voz para el idioma de la página: entonces junto al cuento aparece el botón «Escuchar». La voz es la del propio navegador y no se envía nada a ningún sitio. Puedes pausar, detener y cambiar la velocidad de lectura.",
      "Ja, wenn dein Browser eine Stimme für die Sprache der Seite hat: Dann steht bei der Geschichte die Schaltfläche „Anhören“. Die Stimme ist die deines Browsers, es wird nichts irgendwohin gesendet. Du kannst pausieren, stoppen und die Lesegeschwindigkeit ändern.",
      "Sì, se il tuo browser ha una voce per la lingua della pagina: allora accanto al racconto compare il pulsante «Ascolta». La voce è quella del browser, non viene inviato nulla da nessuna parte. Puoi mettere in pausa, fermare e cambiare la velocità di lettura."),
    [("/tales", TALES)])

add("tale_card", "stories", ["tale"], 550,
    ["How do I make a postcard from a tale?", "Can I save a quote as a picture?",
     "Как сделать открытку из небылицы?", "Можно ли сохранить цитату картинкой?", "Как отправить фразу из небылицы другу?",
     "Як зробити листівку з небилиці?",
     "¿Cómo hago una postal con un cuento?", "¿Puedo guardar una cita como imagen?",
     "Wie mache ich aus einer Geschichte eine Postkarte?", "Kann ich ein Zitat als Bild speichern?",
     "Come faccio una cartolina da un racconto?", "Posso salvare una citazione come immagine?"],
    T("Can I share a line from a tale?", "Можно ли поделиться фразой из небылицы?", "Чи можна поділитися фразою з небилиці?",
      "¿Puedo compartir una frase de un cuento?", "Kann ich eine Zeile aus einer Geschichte teilen?", "Posso condividere una frase di un racconto?"),
    T("Select a phrase in the text (from 12 to 320 characters): a “Make a card” button appears under it. The phrase is set on a picture, either on the photo of the work or on parchment. You can save the picture, share it or copy the link.",
      "Выделите фразу в тексте (от 12 до 320 знаков) — под ней появится «Открытка». Фраза ложится на картинку: на фото работы или на пергамент. Картинку можно сохранить, отправить через «Поделиться» или скопировать ссылку.",
      "Виділіть фразу в тексті (від 12 до 320 знаків) — під нею з’явиться «Листівка». Фраза лягає на картинку: на фото роботи або на пергамент. Картинку можна зберегти, надіслати через «Поділитися» або скопіювати посилання.",
      "Selecciona una frase del texto (de 12 a 320 caracteres): debajo aparece «Hacer una postal». La frase se coloca sobre una imagen, ya sea la foto de la obra o pergamino. Puedes guardar la imagen, compartirla o copiar el enlace.",
      "Markiere im Text eine Zeile (12 bis 320 Zeichen): Darunter erscheint „Postkarte machen“. Die Zeile wird auf ein Bild gesetzt, entweder auf das Foto des Werks oder auf Pergament. Du kannst das Bild speichern, teilen oder den Link kopieren.",
      "Seleziona una frase nel testo (da 12 a 320 caratteri): sotto compare «Crea una cartolina». La frase viene messa su un’immagine, sulla foto dell’opera o su pergamena. Puoi salvare l’immagine, condividerla o copiare il link."),
    [("/tales", TALES)])

add("tale_sequel", "stories", ["tale"], 560,
    ["I want a sequel", "Will there be a continuation of this story?",
     "Хочу продолжение небылицы", "Будет ли продолжение этой истории?", "Как узнать, когда выйдет следующая?",
     "Хочу продовження небилиці",
     "Quiero la continuación del cuento", "¿Habrá segunda parte de esta historia?",
     "Ich will eine Fortsetzung", "Gibt es eine Fortsetzung dieser Geschichte?",
     "Voglio il seguito di questo racconto", "Ci sarà un seguito di questa storia?"],
    T("How do I ask for a sequel to a tale?", "Как попросить продолжение небылицы?", "Як попросити продовження небилиці?",
      "¿Cómo pido la continuación de un cuento?", "Wie bitte ich um eine Fortsetzung einer Geschichte?", "Come chiedo il seguito di un racconto?"),
    T("After the last line of a tale press “I want more”. The news of a continuation goes to a confirmed address; if you signed in through Telegram, you can get it there. A letter goes only to an address whose owner has confirmed it.",
      "После последней строки небылицы нажмите «Хочу продолжение». Письмо о продолжении придёт на подтверждённый адрес; если вы вошли через Telegram, весть можно получить там. Письмо уходит только на адрес, который его владелец подтвердил.",
      "Після останнього рядка небилиці натисніть «Хочу продовження». Лист про продовження прийде на підтверджену адресу; якщо ви ввійшли через Telegram, звістку можна отримати там. Лист іде лише на адресу, яку її власник підтвердив.",
      "Tras la última línea del cuento pulsa «Quiero más». El aviso de la continuación llega a una dirección confirmada; si has entrado con Telegram, puedes recibirlo allí. Solo se escribe a direcciones cuyo dueño las ha confirmado.",
      "Drücke nach der letzten Zeile einer Geschichte auf „Ich will mehr“. Die Nachricht über die Fortsetzung geht an eine bestätigte Adresse; bist du über Telegram angemeldet, kannst du sie dort bekommen. Briefe gehen nur an Adressen, deren Besitzer sie bestätigt hat.",
      "Dopo l’ultima riga di un racconto premi «Voglio il seguito». L’avviso del seguito arriva a un indirizzo confermato; se hai fatto l’accesso con Telegram, puoi riceverlo lì. Le lettere vanno solo a indirizzi il cui proprietario li ha confermati."),
    [("/tales", TALES)])

add("play_dust", "play", [], 570,
    ["How do I earn currency in the game?", "What do I buy cards with?",
     "Чем платить за карты в игре?", "Откуда берётся пыль?", "Как заработать пыль?",
     "Чим платити за карти в грі?",
     "¿Con qué se compran las cartas?", "¿Cómo gano polvo?",
     "Womit kaufe ich Karten im Spiel?", "Wie verdiene ich Staub?",
     "Con che cosa si comprano le carte?", "Come guadagno la polvere?"],
    T("What is dust and how do I earn it?", "Что такое пыль и как её получить?", "Що таке пил і як його отримати?",
      "¿Qué es el polvo y cómo se gana?", "Was ist Staub und wie bekomme ich ihn?", "Che cos’è la polvere e come si guadagna?"),
    T("Dust is the game’s currency. It is earned automatically — for the works you look at and the stories you read on the site — and cards are bought with it. Dust is not money: it cannot be bought or exchanged for money.\n\nAnother in-game currency exists; how it is earned is explained next to your wallet, under “What is this?”.",
      "Пыль — валюта игры. Она начисляется автоматически: за работы, которые вы смотрите, и за истории, которые читаете на сайте. На неё покупают карты. Пыль — не деньги: её нельзя купить за деньги или обменять на деньги.\n\nЕсть и вторая игровая валюта; как её получить, написано рядом с кошельком, в разделе «Что это такое?».",
      "Пил — валюта гри. Він нараховується автоматично: за роботи, які ви дивитеся, і за історії, які читаєте на сайті. За нього купують карти. Пил — не гроші: його не можна купити за гроші чи обміняти на гроші.\n\nЄ й друга ігрова валюта; як її отримати, написано поруч із гаманцем, у розділі «Що це таке?».",
      "El polvo es la moneda del juego. Se gana solo —por las obras que ves y los cuentos que lees en el sitio— y con él se compran cartas. El polvo no es dinero: no se puede comprar con dinero ni cambiar por dinero.\n\nExiste otra moneda del juego; cómo se gana se explica junto a tu saldo, en «¿Qué es esto?».",
      "Staub ist die Währung des Spiels. Du bekommst ihn automatisch — für Werke, die du ansiehst, und Geschichten, die du auf der Seite liest — und kaufst damit Karten. Staub ist kein Geld: Man kann ihn weder für Geld kaufen noch in Geld tauschen.\n\nEs gibt noch eine zweite Spielwährung; wie man sie bekommt, steht neben deinem Guthaben unter „Was ist das?“.",
      "La polvere è la valuta del gioco. Si guadagna da sola — per le opere che guardi e i racconti che leggi sul sito — e con essa si comprano le carte. La polvere non è denaro: non si può comprare con denaro né cambiare in denaro.\n\nEsiste un’altra valuta del gioco; come si ottiene è spiegato accanto al tuo portafoglio, sotto «Che cos’è?»."),
    [("/battles", SHELF)])

add("play_money", "play", [], 580,
    ["Is the game free to play?", "Are there in-app purchases?",
     "Игра платная?", "Есть ли в игре донат?", "Можно ли обменять пыль на деньги?",
     "Гра платна?",
     "¿El juego es gratis?", "¿Hay compras dentro del juego?",
     "Ist das Spiel kostenlos?", "Gibt es Käufe im Spiel?",
     "Il gioco è gratuito?", "Ci sono acquisti nel gioco?"],
    T("Can I buy cards or dust for real money?", "Можно ли купить карты или пыль за деньги?", "Чи можна купити карти або пил за гроші?",
      "¿Puedo comprar cartas o polvo con dinero real?", "Kann ich Karten oder Staub für echtes Geld kaufen?", "Posso comprare carte o polvere con denaro vero?"),
    T("No. Dust and the other in-game currencies are not money: they have no cash value, cannot be bought for money or exchanged into it, and exist only inside the game. Cards are bought with dust, which is earned for the works you look at and the stories you read.",
      "Нет. Пыль и другие игровые валюты — не деньги: у них нет денежной стоимости, их нельзя купить за деньги или обменять на деньги, они существуют только внутри игры. Карты покупают за пыль, а пыль начисляется за работы, которые вы смотрите, и истории, которые читаете.",
      "Ні. Пил та інші ігрові валюти — не гроші: вони не мають грошової вартості, їх не можна купити за гроші чи обміняти на гроші, вони існують лише всередині гри. Карти купують за пил, а пил нараховується за роботи, які ви дивитеся, та історії, які читаєте.",
      "No. El polvo y las demás monedas del juego no son dinero: no tienen valor en dinero, no se pueden comprar con dinero ni cambiar por dinero, y existen solo dentro del juego. Las cartas se compran con polvo, que se gana por las obras que ves y los cuentos que lees.",
      "Nein. Staub und die anderen Spielwährungen sind kein Geld: Sie haben keinen Geldwert, lassen sich weder für Geld kaufen noch in Geld tauschen und existieren nur im Spiel. Karten kauft man mit Staub, den man für Werke bekommt, die man ansieht, und Geschichten, die man liest.",
      "No. La polvere e le altre valute del gioco non sono denaro: non hanno valore in denaro, non si possono comprare con denaro né cambiare in denaro, ed esistono solo dentro il gioco. Le carte si comprano con la polvere, che si guadagna per le opere che guardi e i racconti che leggi."))

add("studio_what", "studio", [], 590,
    ["Can I make my own card frame?", "How does the weekly frame vote work?", "What is the Studio section for?",
     "Можно ли сделать свою рамку для карт?", "Как работает недельное голосование за рамки?", "Для чего раздел «Студия»?",
     "Чи можна зробити власну рамку для карт?",
     "¿Puedo hacer mi propio marco de carta?", "¿Cómo funciona la votación semanal de marcos?",
     "Kann ich einen eigenen Kartenrahmen machen?", "Wie funktioniert die wöchentliche Abstimmung über Rahmen?",
     "Posso fare una mia cornice per le carte?", "Come funziona il voto settimanale sulle cornici?"],
    T("What is the studio?", "Что такое студия?", "Що таке студія?",
      "¿Qué es el estudio?", "Was ist das Studio?", "Che cos’è lo studio?"),
    T("The studio is where people make frames for the game’s cards. A week of frames made by visitors is marked from one to five; on Sunday the votes are counted and three go to the author for approval. A frame you make remains your work, and a licence to a frame is the right to wear it in the game, not ownership of the artwork or its copyright.",
      "Студия — место, где люди делают рамки для карт игры. Неделя рамок, сделанных посетителями, получает оценки от единицы до пяти; в воскресенье голоса считаются, и три работы уходят автору на утверждение. Сделанная вами рамка остаётся вашей работой, а лицензия на рамку — это право носить её в игре, а не право собственности на рисунок или авторские права.",
      "Студія — місце, де люди роблять рамки для карт гри. Тиждень рамок, зроблених відвідувачами, отримує оцінки від одиниці до п’яти; у неділю голоси підраховуються, і три роботи йдуть авторові на затвердження. Зроблена вами рамка залишається вашою роботою, а ліцензія на рамку — це право носити її в грі, а не право власності на малюнок чи авторські права.",
      "El estudio es el lugar donde la gente hace marcos para las cartas del juego. Una semana de marcos hechos por visitantes recibe notas de uno a cinco; el domingo se cuentan los votos y tres pasan a la autora para su aprobación. Un marco que haces sigue siendo tu obra, y una licencia de marco es el derecho a llevarlo en el juego, no la propiedad del dibujo ni de sus derechos de autor.",
      "Im Studio machen Besucher Rahmen für die Karten des Spiels. Eine Woche selbstgemachter Rahmen wird mit Noten von eins bis fünf bewertet; am Sonntag werden die Stimmen gezählt, und drei gehen zur Bestätigung an die Autorin. Ein Rahmen, den du machst, bleibt dein Werk, und eine Lizenz für einen Rahmen ist das Recht, ihn im Spiel zu tragen — kein Eigentum an der Zeichnung und kein Urheberrecht.",
      "Lo studio è il posto dove le persone fanno cornici per le carte del gioco. Una settimana di cornici fatte dai visitatori riceve voti da uno a cinque; la domenica si contano i voti e tre passano all’autrice per l’approvazione. Una cornice che fai resta una tua opera, e una licenza di cornice è il diritto di portarla nel gioco, non la proprietà del disegno né del suo diritto d’autore."),
    [("/studio", STUDIO)])

# Черновики: чего сайт не говорит. Вопрос и фразы есть, ответ пишет автор в админке.
add("visit_workshop", "order", [], 600,
    ["Can I visit the workshop?", "Can I see a work in person before choosing?",
     "Можно ли приехать в мастерскую?", "Можно ли посмотреть работу вживую?",
     "Чи можна приїхати до майстерні?",
     "¿Puedo visitar el taller?", "¿Puedo ver una obra en persona?",
     "Kann ich die Werkstatt besuchen?", "Kann ich ein Werk persönlich ansehen?",
     "Posso visitare il laboratorio?", "Posso vedere un’opera dal vivo?"],
    T("Can I visit the workshop or see a work in person?", "Можно ли приехать в мастерскую или посмотреть работу вживую?", "Чи можна приїхати до майстерні або подивитися роботу наживо?",
      "¿Puedo visitar el taller o ver una obra en persona?", "Kann ich die Werkstatt besuchen oder ein Werk persönlich ansehen?", "Posso visitare il laboratorio o vedere un’opera dal vivo?"),
    {}, enabled=False)

add("repair", "order", [], 610,
    ["Do you repair or restore figurines?", "My figurine is damaged, can it be repaired?",
     "Делаете ли вы ремонт или реставрацию фигурок?", "Фигурка повредилась, её можно починить?",
     "Чи ремонтуєте ви фігурки?",
     "¿Reparáis o restauráis figuras?", "Mi figura se ha dañado, ¿se puede reparar?",
     "Repariert oder restauriert ihr Figuren?", "Meine Figur ist beschädigt, kann sie repariert werden?",
     "Riparate o restaurate le statuine?", "La mia statuina si è rovinata, si può riparare?"],
    T("Do you repair figurines?", "Делаете ли вы ремонт фигурок?", "Чи ремонтуєте ви фігурки?",
      "¿Reparáis figuras?", "Repariert ihr Figuren?", "Riparate le statuine?"),
    {}, enabled=False)

add("certificate", "works", [], 620,
    ["Is there a certificate of authenticity?", "Does a work come with a certificate?",
     "Есть ли сертификат подлинности?", "Прилагается ли к работе сертификат?",
     "Чи є сертифікат автентичності?",
     "¿Hay certificado de autenticidad?", "¿La obra viene con certificado?",
     "Gibt es ein Echtheitszertifikat?", "Wird ein Zertifikat mitgeliefert?",
     "C’è un certificato di autenticità?", "L’opera è accompagnata da un certificato?"],
    T("Does a work come with a certificate of authenticity?", "Есть ли у работы сертификат подлинности?", "Чи є в роботи сертифікат автентичності?",
      "¿La obra viene con certificado de autenticidad?", "Wird ein Werk mit Echtheitszertifikat geliefert?", "L’opera è accompagnata da un certificato di autenticità?"),
    {}, enabled=False)

add("customs", "order", [], 630,
    ["Who pays customs and duties?", "Will I pay import tax when it ships abroad?",
     "Кто платит таможенные пошлины?", "Придётся ли платить налог при доставке за границу?",
     "Хто платить мито?",
     "¿Quién paga la aduana y los aranceles?", "¿Tendré que pagar impuestos de importación?",
     "Wer zahlt Zoll und Gebühren?", "Muss ich Einfuhrsteuer zahlen?",
     "Chi paga la dogana e i dazi?", "Dovrò pagare tasse di importazione?"],
    T("Who pays customs and duties when a work ships abroad?", "Кто платит таможню и пошлины при доставке за границу?", "Хто платить митницю й мито під час доставки за кордон?",
      "¿Quién paga la aduana y los aranceles en un envío al extranjero?", "Wer zahlt Zoll und Gebühren bei einem Versand ins Ausland?", "Chi paga dogana e dazi per una spedizione all’estero?"),
    {}, enabled=False)

add("custom_size", "order", [], 640,
    ["Can a work be made in a different size?", "Can you make it bigger or smaller?",
     "Можно ли сделать работу другого размера?", "Можно ли сделать крупнее или меньше?",
     "Чи можна зробити роботу іншого розміру?",
     "¿Se puede hacer una obra de otro tamaño?", "¿Podéis hacerla más grande o más pequeña?",
     "Kann ein Werk in einer anderen Größe gemacht werden?", "Könnt ihr es größer oder kleiner machen?",
     "Si può fare un’opera di un’altra misura?", "Potete farla più grande o più piccola?"],
    T("Can a work be made in a different size?", "Можно ли сделать работу другого размера?", "Чи можна зробити роботу іншого розміру?",
      "¿Se puede hacer una obra de otro tamaño?", "Kann ein Werk in einer anderen Größe gemacht werden?", "Si può fare un’opera di un’altra misura?"),
    {}, enabled=False)

# Включается из черновика: сайт сам говорит, чем покупают карты.
CARDS_GET = dict(
    answer=T("Cards are bought with dust. Dust is earned automatically — for the works you look at and the stories you read on the site. The card shelf shows which cards you can take now.",
             "Карты покупают за пыль. Пыль начисляется автоматически: за работы, которые вы смотрите, и за истории, которые читаете на сайте. Полка карт показывает, какие карты вы уже можете взять.",
             "Карти купують за пил. Пил нараховується автоматично: за роботи, які ви дивитеся, і за історії, які читаєте на сайті. Полиця карт показує, які карти ви вже можете взяти.",
             "Las cartas se compran con polvo. El polvo se gana solo —por las obras que ves y los cuentos que lees en el sitio—. El estante de cartas muestra qué cartas puedes tomar ya.",
             "Karten kauft man mit Staub. Staub bekommst du automatisch — für Werke, die du ansiehst, und Geschichten, die du auf der Seite liest. Das Kartenregal zeigt, welche Karten du schon nehmen kannst.",
             "Le carte si comprano con la polvere. La polvere si guadagna da sola — per le opere che guardi e i racconti che leggi sul sito. Lo scaffale delle carte mostra quali carte puoi già prendere."),
    links=[("/battles", SHELF)],
)


# ── Сборка SQL ───────────────────────────────────────────────────────────────
def lit(value):
    text = json.dumps(value, ensure_ascii=False)
    assert "$q$" not in text and "$o$" not in text and "$n$" not in text
    return "$q$" + text + "$q$::jsonb"


def link_json(href, label):
    return {"href": href, "label": label}


def first_batch():
    """key -> links, как они лежат в первой миграции."""
    text = FIRST.read_text(encoding="utf-8")
    found = {}
    pattern = re.compile(
        r"\(\$k\$(\w+)\$k\$, (?:NULL|\$k\$\w+\$k\$), (?:TRUE|FALSE), \d+, \$q\$.*?\$q\$::jsonb, \$q\$.*?\$q\$::jsonb, \$q\$.*?\$q\$::jsonb, \$q\$(.*?)\$q\$::jsonb\)",
        re.S,
    )
    for m in pattern.finditer(text):
        found[m.group(1)] = json.loads(m.group(2))
    return found


def keep_filled(column):
    """Выражение: уже заполненные (не пустые) языки столбца."""
    return (
        f"(SELECT COALESCE(jsonb_object_agg(e.k, e.v), '{{}}'::jsonb) "
        f"FROM jsonb_each({column}) AS e(k, v) WHERE e.v <> '\"\"'::jsonb)"
    )


def main():
    name = sys.argv[1] if len(sys.argv) > 1 else "20261023000001_ask_more_answers.sql"
    old_links = first_batch()
    parts = []

    parts.append("-- 1. Переводы на испанский, немецкий и итальянский к записям первой партии.\n"
                 "-- Заполняются только пустые языки: то, что правили в админке, остаётся.\n")
    for key, (q, a) in FILL.items():
        sets = [f"question = {lit(q)} || {keep_filled('question')}"]
        if a:
            sets.append(f"answer = {lit(a)} || {keep_filled('answer')}")
        links = old_links.get(key)
        if links:
            new_links = []
            for l in links:
                tail_labels = LABEL_TAIL_BY_KEY.get((key, l["href"])) or LABEL_TAIL.get(l["href"])
                if tail_labels is None:
                    raise SystemExit(f"нет подписи ссылки {l['href']} у {key}")
                label = dict(l["label"])
                for lang in ("es", "de", "it"):
                    label[lang] = tail_labels[lang]
                new_links.append({"href": l["href"], "label": label})
            sets.append(f"links = CASE WHEN links = {lit(links)} THEN {lit(new_links)} ELSE links END")
        parts.append("UPDATE ask_answers SET " + ", ".join(sets) + f", updated_at = now() WHERE key = '{key}';")

    parts.append("\n-- 2. Новая партия записей: тема, страницы, фразы, вопрос и ответ на шести языках.\n"
                 "-- Выключенные — черновики: вопрос узнаётся, а ответ пишет автор.\n")
    rows = []
    for e in NEW:
        links = [link_json(h, lab) for h, lab in e["links"]]
        rows.append(
            "  ($k${key}$k$, NULL, {enabled}, {order}, {topic}, {pages}, {ph}, {q}, {a}, {l})".format(
                key=e["key"],
                enabled="TRUE" if e["enabled"] else "FALSE",
                order=e["order"],
                topic="'" + e["topic"] + "'",
                pages=lit(e["pages"]),
                ph=lit(e["phrases"]),
                q=lit(e["question"]),
                a=lit(e["answer"]),
                l=lit(links),
            )
        )
        if e["enabled"]:
            assert len(e["phrases"]) >= 3 and e["question"]["en"] and e["question"]["ru"] and e["answer"]["en"] and e["answer"]["ru"], e["key"]
            assert set(e["question"]) == set(LANGS) and set(e["answer"]) == set(LANGS), e["key"]
    parts.append(
        "INSERT INTO ask_answers (key, handler, enabled, sort_order, topic, pages, phrases, question, answer, links) VALUES\n"
        + ",\n".join(rows)
        + "\nON CONFLICT (key) DO NOTHING;"
    )

    parts.append("\n-- 3. «Как получить новые карты?» перестаёт быть черновиком: сайт сам говорит,\n"
                 "-- что карты покупают за пыль. Только если автор ещё ничего в нём не писал.\n")
    parts.append(
        "UPDATE ask_answers SET enabled = TRUE, answer = {a}, links = {l}, updated_at = now() "
        "WHERE key = 'play_cards_get' AND enabled = FALSE AND answer = '{{}}'::jsonb;".format(
            a=lit(CARDS_GET["answer"]),
            l=lit([link_json(h, lab) for h, lab in CARDS_GET["links"]]),
        )
    )

    sql = (
        "-- Каталог «Спросить сайт», вторая партия: новые ответы и переводы старых.\n"
        "-- Сгенерировано tools/ask/make_more_migration.py; применённые миграции не правят.\n"
        "-- Что сделано и откуда взяты тексты — в шапке самого скрипта.\n\n"
        + "\n".join(parts)
        + "\n"
    )
    (OUT_DIR / name).write_text(sql, encoding="utf-8")
    print(f"{OUT_DIR / name}: {len(FILL)} дополнено, {len(NEW)} новых, {sum(1 for e in NEW if e['enabled'])} включено")


main()
