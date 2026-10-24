#!/usr/bin/env python3
"""Собирает миграцию каталога «Спросить сайт» из tools/ask/catalog.json.

Формулировки берутся из catalog.json (их же меряет examples/ask_probe.rs);
тексты ответов лежат здесь и взяты только из того, что сайт уже говорит сам
(страница «Как приобрести работу», окно «Спросить сайт» и т. д.). Чего сайт не
говорит (сроки изготовления, уход, скидки, подарки, откуда берутся карты),
записи заводятся выключенными и без ответа: ответ пишет автор в админке.

Применённые миграции не правят (sqlx сверяет контрольную сумму). Новый набор —
новая миграция:  python3 tools/ask/make_migration.py ИМЯ_ФАЙЛА.sql
"""
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
OUT_DIR = HERE.parent.parent / "src-tauri" / "server" / "migrations"

# key -> (en, ru, uk, [(href, en, ru, uk)])
A = {}

ACQ = ("/acquire", "How to acquire a work", "Как приобрести работу", "Як придбати роботу")
COM = ("/commission", "Commission a work", "Заказать работу", "Замовити роботу")
ALL = ("/figurines", "All works", "Все работы", "Усі роботи")

A["how_order"] = (
    "Choose a finished work and send a request, or commission a new one. You write to the master directly; nothing is charged and nothing is owed until you both agree. If the master takes the work, they write to you personally with the terms.",
    "Выберите готовую работу и отправьте запрос — или закажите новую. Вы пишете мастеру напрямую; пока вы не договоритесь, ничего не списывается и ни к чему не обязывает. Если мастер берётся за работу, он лично пишет вам с условиями.",
    "Оберіть готову роботу й надішліть запит — або замовте нову. Ви пишете майстру напряму; доки ви не домовитеся, нічого не списується і ні до чого не зобов’язує. Якщо майстер береться за роботу, він особисто пише вам з умовами.",
    [ACQ, COM, ALL],
)
A["price"] = (
    "There are no prices on the site: the master names the terms personally, in a message. Nothing is charged until you both agree.",
    "Цен на сайте нет: условия мастер называет лично в переписке. Пока вы не договоритесь, ничего не списывается.",
    "Цін на сайті немає: умови майстер називає особисто в листуванні. Доки ви не домовитеся, нічого не списується.",
    [ACQ],
)
A["payment"] = (
    "A non-refundable deposit secures the piece and covers materials and time. The balance is settled before the work leaves the workshop. The master names the terms personally.",
    "Невозвратный депозит закрепляет работу и покрывает материалы и время. Остаток оплачивается до того, как работа покинет мастерскую. Условия мастер называет лично.",
    "Безповоротний завдаток закріплює роботу, покриває матеріали й час. Решта сплачується до того, як робота залишить майстерню. Умови майстер називає особисто.",
    [ACQ],
)
A["shipping"] = (
    "Delivery is insured, tracked, and one-way — packed by hand and photographed before it travels. The balance is settled before the work leaves the workshop.",
    "Доставка застрахованная, с отслеживанием и в одну сторону — работа упакована вручную и сфотографирована перед отправкой. Остаток оплачивается до того, как она покинет мастерскую.",
    "Доставка застрахована, з відстеженням і в один бік — робота запакована вручну й сфотографована перед відправленням. Решта сплачується до того, як вона залишить майстерню.",
    [ACQ],
)
A["returns"] = (
    "Each piece is fragile and one of a kind, so all sales are final and returns are not possible. Delivery is insured, and every work is photographed before it travels.",
    "Каждая работа хрупкая и единственная в своём роде, поэтому все продажи финальные и возврат невозможен. Доставка застрахована, а перед отправкой работа фотографируется.",
    "Кожна робота крихка й єдина у своєму роді, тому всі продажі фінальні й повернення неможливе. Доставка застрахована, а перед відправленням робота фотографується.",
    [ACQ],
)
A["order_status"] = (
    "Delivery is tracked. For a commission, you may follow its progress and speak with the master along the way. Your requests and the replies are kept in your profile.",
    "Доставка отслеживается. Если работа на заказ, вы можете следить за ходом и общаться с мастером по пути. Ваши запросы и ответы мастера лежат в профиле.",
    "Доставка відстежується. Якщо робота на замовлення, ви можете стежити за ходом і спілкуватися з майстром дорогою. Ваші запити й відповіді майстра лежать у профілі.",
    [("/profile/dealings", "My requests", "Мои обращения", "Мої звернення")],
)
A["commission"] = (
    "Yes. Send a request to commission a new work and write to the master directly: nothing is charged until you both agree. For a commission the figure takes shape by hand, one stage at a time, and you may follow its progress.",
    "Да. Отправьте запрос на новую работу и пишите мастеру напрямую: пока вы не договоритесь, ничего не списывается. Для заказа фигура рождается вручную, шаг за шагом, и вы можете следить за ходом работы.",
    "Так. Надішліть запит на нову роботу й пишіть майстру напряму: доки ви не домовитеся, нічого не списується. Для замовлення фігура народжується вручну, крок за кроком, і ви можете стежити за ходом роботи.",
    [COM, ACQ],
)
A["reserve"] = (
    "If the master takes the work, they write to you personally with the terms. A non-refundable deposit secures the piece.",
    "Если мастер берётся за работу, он лично пишет вам с условиями. Невозвратный депозит закрепляет работу.",
    "Якщо майстер береться за роботу, він особисто пише вам з умовами. Безповоротний завдаток закріплює роботу.",
    [ACQ],
)
A["contact"] = (
    "You can write to the author with the “Write” button in the header or the form on the home page. The author reads every message and answers personally, usually within a day.",
    "Написать автору можно кнопкой «Написать» в шапке или формой на главной странице. Автор читает каждое письмо и отвечает лично, обычно в течение дня.",
    "Написати авторові можна кнопкою «Написати» в шапці або формою на головній сторінці. Автор читає кожен лист і відповідає особисто, зазвичай протягом дня.",
    [("/author", "About the author", "Об авторе", "Про автора"), ACQ],
)
A["author"] = (
    "Every figurine is made by one author, who sculpts, paints and dresses each by hand, so no two are alike.",
    "Все фигурки делает один автор: каждую он лепит, расписывает и одевает вручную, поэтому двух одинаковых нет.",
    "Усі фігурки робить один автор: кожну він ліпить, розписує й одягає вручну, тому двох однакових немає.",
    [("/author", "About the author", "Об авторе", "Про автора"), ("/workshop", "The workshop", "Мастерская", "Майстерня")],
)
A["materials"] = (
    "The figurines are sculpted, painted and dressed by hand, and each exists in a single copy. The material and technique of each work are written on its page.",
    "Фигурки лепят, расписывают и одевают вручную, и каждая существует в единственном экземпляре. Материал и техника каждой работы указаны на её странице.",
    "Фігурки ліплять, розписують і одягають вручну, і кожна існує в єдиному екземплярі. Матеріал і техніка кожної роботи вказані на її сторінці.",
    [("/workshop", "The workshop", "Мастерская", "Майстерня"), ALL],
)
A["size"] = (
    "The exact size is written on the page of each work.",
    "Точный размер указан на странице каждой работы.",
    "Точний розмір указано на сторінці кожної роботи.",
    [ALL],
)
A["unique"] = (
    "Each work is one of a kind and exists in a single copy.",
    "Каждая работа единственная в своём роде и существует в одном экземпляре.",
    "Кожна робота єдина у своєму роді й існує в одному екземплярі.",
    [ALL],
)
A["is_shop"] = (
    "This is a showcase of original gothic miniatures, not a shop. There is no cart and no checkout: to acquire a work, you write to the master.",
    "Это витрина авторских готических миниатюр, а не магазин. Ни корзины, ни оформления: чтобы получить работу, вы пишете мастеру.",
    "Це вітрина авторських готичних мініатюр, а не магазин. Ні кошика, ні оформлення: щоб отримати роботу, ви пишете майстру.",
    [ACQ, ALL],
)
A["play_rules"] = (
    "A study is a game against the computer on a small field. You place your cards and take turns; whoever still has cards when the opponent has none wins.\n\nEach study may have its own rules: they are written on the battle sheet before it starts. The first battles can be played without signing in.",
    "Этюд — партия против компьютера на небольшом поле. Вы расставляете карты и ходите по очереди; выигрывает тот, у кого остались карты, когда у противника их не осталось.\n\nУ каждого этюда могут быть свои правила: они написаны на листе боя перед началом. Первые бои можно играть без входа.",
    "Етюд — партія проти комп’ютера на невеликому полі. Ви розставляєте карти й ходите по черзі; виграє той, у кого залишилися карти, коли в противника їх не залишилося.\n\nУ кожного етюду можуть бути свої правила: вони написані на аркуші бою перед початком. Перші бої можна грати без входу.",
    [("/battles/etude", "Studies", "Этюды", "Етюди"), ("/battles", "Card shelf", "Полка карт", "Полиця карт")],
)
A["play_first"] = (
    "Open the studies and pick the first battle. The first battles can be played without signing in, and before each one the battle sheet tells its rules.",
    "Откройте этюды и выберите первый бой. Первые бои можно играть без входа, а перед каждым на листе боя написаны его правила.",
    "Відкрийте етюди й оберіть перший бій. Перші бої можна грати без входу, а перед кожним на аркуші бою написані його правила.",
    [("/battles/etude", "Studies", "Этюды", "Етюди")],
)
A["play_win"] = (
    "You win when you still have cards and the opponent has none. Some studies add rules of their own; they are written on the battle sheet before the start.",
    "Вы выигрываете, когда у вас остались карты, а у противника их нет. В некоторых этюдах есть свои правила: они написаны на листе боя перед началом.",
    "Ви виграєте, коли у вас залишилися карти, а в противника їх немає. У деяких етюдах є свої правила: вони написані на аркуші бою перед початком.",
    [("/battles/etude", "Studies", "Этюды", "Етюди")],
)
A["play_races"] = (
    "Every race hits some races harder and is protected from others: +1 damage in blows and spells against the marked races, and 1 less damage taken from the others. It is written on the sheet of each card, in the section about the race.",
    "У каждой расы есть те, против кого она сильнее (+1 к урону от ударов и чар), и те, от кого она защищена (−1 получаемого урона). Это написано на листе каждой карты, в разделе «Раса в бою».",
    "У кожної раси є ті, проти кого вона сильніша (+1 до шкоди від ударів і чар), і ті, від кого вона захищена (−1 отриманої шкоди). Це написано на аркуші кожної карти, у розділі «Раса в бою».",
    [("/battles", "Card shelf", "Полка карт", "Полиця карт")],
)
A["tale_what"] = (
    "A tall tale is a short story about a figurine. Many have a work of their own, and the work has a card in the game.",
    "Небылица — короткая история о фигурке. У многих небылиц есть своя работа, а у работы — карта в игре.",
    "Небилиця — коротка історія про фігурку. У багатьох небилиць є своя робота, а в роботи — карта в грі.",
    [("/tales", "Tall tales", "Небылицы", "Небилиці")],
)
A["gazette_what"] = (
    "The gazette is a feed of short leaves: what has arrived in the workshop, and news around miniatures. Tall tales are leaves too, but they have their own shelf.",
    "Вестник — лента коротких листов: о том, что появилось в мастерской, и новости вокруг миниатюр. Небылицы — тоже листы, но у них своя полка.",
    "Вісник — стрічка коротких аркушів: про те, що з’явилося в майстерні, і новини довкола мініатюр. Небилиці — теж аркуші, але в них своя полиця.",
    [("/gazette", "Gazette", "Вестник", "Вісник"), ("/tales", "Tall tales", "Небылицы", "Небилиці")],
)
A["unsubscribe"] = (
    "Every email from the house carries an unsubscribe link. In the Telegram bot, send /stop.",
    "В каждом письме от дома есть ссылка для отказа от рассылки. В боте в Telegram напишите /stop.",
    "У кожному листі від дому є посилання для відмови від розсилки. У боті в Telegram напишіть /stop.",
    [],
)
A["privacy"] = (
    "What the site stores about visitors, and for how long, is written on the privacy page.",
    "Что сайт хранит о посетителях и как долго, написано на странице о конфиденциальности.",
    "Що сайт зберігає про відвідувачів і як довго, написано на сторінці про конфіденційність.",
    [("/privacy", "Privacy", "Конфиденциальность", "Конфіденційність")],
)
A["account"] = (
    "You can browse the site and play the first battles without signing in. To sign in, use your email with four symbols, Telegram, or Google.",
    "Смотреть сайт и играть в первые бои можно без входа. Войти можно по почте и четырём символам, через Telegram или Google.",
    "Дивитися сайт і грати в перші бої можна без входу. Увійти можна за поштою й чотирма символами, через Telegram або Google.",
    [("/login", "Sign in", "Войти", "Увійти"), ("/register", "Create a name", "Завести имя", "Завести ім’я")],
)
A["forgot_signs"] = (
    "On the sign-in page choose “Forgot your symbols?”. A link to choose new symbols goes to every way back you have confirmed: your confirmed email and Telegram, if it is linked. The link lives two hours.",
    "На странице входа выберите «Забыли символы?». Ссылка, чтобы выбрать новые символы, придёт по каждому подтверждённому пути назад: на подтверждённую почту и в Telegram, если он привязан. Ссылка живёт два часа.",
    "На сторінці входу оберіть «Забули символи?». Посилання, щоб обрати нові символи, прийде кожним підтвердженим шляхом назад: на підтверджену пошту й у Telegram, якщо він прив’язаний. Посилання живе дві години.",
    [("/login", "Sign in", "Войти", "Увійти")],
)
A["bot_nature"] = (
    "Neither a person nor a neural network: this window picks an answer by the meaning of your question from ready-made answers and from the site’s own data.",
    "Это не человек и не нейросеть: окно подбирает ответ по смыслу вопроса из готовых ответов и из данных самого сайта.",
    "Це не людина й не нейромережа: вікно добирає відповідь за змістом запитання з готових відповідей і з даних самого сайту.",
    [("/author", "Write to the master", "Написать мастеру", "Написати майстру")],
)

# Чего сайт не говорит сам: записи заводятся выключенными, без ответа.
DRAFTS = {"lead_time", "care", "discounts", "gift", "play_cards_get"}
HANDLERS = {"data_count": "count", "data_new": "new", "data_read": "read", "data_start": "start"}


def lit(value):
    return "$q$" + json.dumps(value, ensure_ascii=False) + "$q$::jsonb"


def main():
    name = sys.argv[1] if len(sys.argv) > 1 else "20261022000004_ask_answers.sql"
    catalog = json.loads((HERE / "catalog.json").read_text(encoding="utf-8"))
    rows = []
    for order, entry in enumerate(catalog):
        key = entry["id"]
        ph = entry["phrases"]
        handler = HANDLERS.get(key)
        question = {"en": ph[0], "ru": ph[3], "uk": ph[7]}
        answer = {}
        links = []
        enabled = False
        if handler:
            enabled = True
        elif key in A:
            en, ru, uk, ls = A[key]
            answer = {"en": en, "ru": ru, "uk": uk}
            links = [{"href": h, "label": {"en": e, "ru": r, "uk": u}} for h, e, r, u in ls]
            enabled = True
        elif key not in DRAFTS:
            raise SystemExit(f"нет ответа и не черновик: {key}")
        rows.append(
            "  ($k${key}$k$, {handler}, {enabled}, {order}, {ph}, {q}, {a}, {l})".format(
                key=key,
                handler=f"$k${handler}$k$" if handler else "NULL",
                enabled="TRUE" if enabled else "FALSE",
                order=(order + 1) * 10,
                ph=lit(ph),
                q=lit(question),
                a=lit(answer),
                l=lit(links),
            )
        )
    sql = f"""-- Каталог ответов «Спросить сайт» (`ask_answers`).
--
-- Окно узнаёт вопрос по смыслу среди записей этого каталога (вектор той же
-- модели, что ищет работы, и подтверждение поиском по словам) и отвечает
-- записью; не уверено — предлагает три ближайших вопроса кнопками. Каталог
-- правится в админке. Сгенерировано tools/ask/make_migration.py из
-- tools/ask/catalog.json; применённые миграции не правят.
--
-- Тексты языков лежат одним JSON (`question`, `answer`), а не колонками
-- `_en … _it`: запись правится целиком и по языку никогда не выбирается.
-- Обязательны английский и русский, остальные откатываются по цепочке языков.

CREATE TABLE IF NOT EXISTS ask_answers (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Постоянное имя записи.
    key        TEXT NOT NULL UNIQUE CHECK (key ~ '^[a-z][a-z0-9_]{{1,39}}$'),
    -- NULL — готовый текст; иначе ответ собирает страница из данных сайта.
    handler    TEXT CHECK (handler IN ('count', 'new', 'read', 'start')),
    -- Выключенная запись в подборе не участвует (черновик без ответа).
    enabled    BOOLEAN NOT NULL DEFAULT FALSE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    -- Как люди спрашивают, на любых языках вперемешку: ["…", "…"].
    phrases    JSONB NOT NULL DEFAULT '[]'::jsonb CHECK (jsonb_typeof(phrases) = 'array'),
    -- Вопрос, как он напечатан на кнопке: {{"en": "…", "ru": "…"}}.
    question   JSONB NOT NULL DEFAULT '{{}}'::jsonb CHECK (jsonb_typeof(question) = 'object'),
    answer     JSONB NOT NULL DEFAULT '{{}}'::jsonb CHECK (jsonb_typeof(answer) = 'object'),
    -- Ссылки под ответом: [{{"href": "/acquire", "label": {{"en": "…", "ru": "…"}}}}].
    links      JSONB NOT NULL DEFAULT '[]'::jsonb CHECK (jsonb_typeof(links) = 'array'),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO ask_answers (key, handler, enabled, sort_order, phrases, question, answer, links) VALUES
{(","+chr(10)).join(rows)}
ON CONFLICT (key) DO NOTHING;
"""
    (OUT_DIR / name).write_text(sql, encoding="utf-8")
    print(f"{OUT_DIR / name}: {len(rows)} записей")


main()
