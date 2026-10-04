#!/usr/bin/env python3
"""Вырезает фигурки из фотографий: фигурка остаётся, фон уходит.

Запускать через обёртку `tools/cutout/cutout` — она сама поставит всё нужное
при первом запуске. Подробно — `tools/cutout/README.md`.
"""

from __future__ import annotations

import argparse
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import engine  # noqa: E402
from batch import Batch, BatchError, human_time, is_result_dir, short  # noqa: E402
from store import Busy, FolderLock  # noqa: E402

# ── вывод ────────────────────────────────────────────────────────────────

COLOR = sys.stdout.isatty() and not os.environ.get("NO_COLOR")


def paint(text: str, code: str) -> str:
    return f"\033[{code}m{text}\033[0m" if COLOR else text


def dim(t): return paint(t, "2")
def warn(t): return paint(t, "33")
def bad(t): return paint(t, "31")
def bold(t): return paint(t, "1")


# ── команды ──────────────────────────────────────────────────────────────

EXAMPLES = """
примеры:
  cutout ~/Desktop/ведьма.jpg                 один файл
  cutout ~/Desktop/фото                       вся папка с подпапками
  cutout *.jpg -o ~/Desktop/готово            несколько файлов, своя папка результата
  cutout фото --bg "#f8f1e7" --format jpg     на пергаменте сайта, в JPEG
  cutout фото --crop --square --max-size 1200 обрезать по фигурке, квадрат 1200 px
  cutout фото --fast                          быстрый черновик (хуже края)
  cutout review фото-cutout                   проверить и поправить щелчком

Повторный запуск той же команды продолжает с места: готовое пропускается.
Смена фона, обрезки или формата не пересчитывает нейросеть — файл
пересобирается из сохранённой маски за пару секунд.
"""


def parse_run(argv: list[str]) -> argparse.Namespace:
    p = argparse.ArgumentParser(
        prog="cutout",
        usage="cutout                      окно программы (проще всего)\n       cutout ФАЙЛ_ИЛИ_ПАПКА [...] [параметры]   обработка в терминале\n       cutout review ПАПКА_РЕЗУЛЬТАТОВ          открыть результат в окне\n       cutout --install            значок на рабочем столе",
        description="Вырезает фигурки с фотографий: фигурка остаётся, фон становится прозрачным.",
        epilog=EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
        add_help=False,
    )
    p._positionals.title = "что обрабатывать"
    p._optionals.title = "куда"
    p.add_argument("inputs", nargs="+", metavar="ФАЙЛ_ИЛИ_ПАПКА",
                   help="файлы и/или папки; форматы: " + ", ".join(sorted(e.lstrip('.') for e in engine.INPUT_EXTS)))
    p.add_argument("-o", "--out", metavar="ПАПКА",
                   help="куда класть результат (по умолчанию: рядом с папкой, «<папка>-cutout»)")
    p.add_argument("--no-subfolders", action="store_true", help="не заходить в подпапки")

    g = p.add_argument_group("каким получится файл")
    g.add_argument("-f", "--format", choices=["png", "webp", "jpg"], default="png",
                   help="png (по умолчанию, без потерь), webp (легче в разы, с прозрачностью), jpg (только с фоном)")
    g.add_argument("--bg", metavar="ЦВЕТ",
                   help="залить фон цветом вместо прозрачности: \"#f8f1e7\", white, black")
    g.add_argument("--crop", action="store_true", help="обрезать по фигурке")
    g.add_argument("--pad", type=float, default=5.0, metavar="%",
                   help="поле вокруг фигурки при обрезке, %% её размера (по умолчанию 5)")
    g.add_argument("--square", action="store_true", help="квадратный холст (включает --crop)")
    g.add_argument("--max-size", type=int, default=0, metavar="PX", help="ограничить длинную сторону")
    g.add_argument("--quality", type=int, default=92, metavar="1-100", help="качество webp/jpg (по умолчанию 92)")
    g.add_argument("--mask", action="store_true", help="положить рядом чёрно-белые маски (папка masks/)")

    g = p.add_argument_group("как резать")
    g.add_argument("--model", choices=list(engine.MODELS), default=engine.DEFAULT_MODEL,
                   help="; ".join(f"{k} — {v}" for k, v in engine.MODELS.items()))
    g.add_argument("--fast", action="store_true", help=f"то же, что --model {engine.FAST_MODEL}")
    g.add_argument("--raw-edges", action="store_true",
                   help="не очищать края от цвета старого фона (по умолчанию очищаются)")
    g.add_argument("--keep-specks", action="store_true",
                   help="не убирать мелкие отдельные крошки (по умолчанию убираются)")

    g = p.add_argument_group("повторный запуск")
    g.add_argument("--overwrite", action="store_true",
                   help="пересчитать всё заново (исправленное вручную не трогается)")
    g.add_argument("--discard-edits", action="store_true",
                   help="вместе с --overwrite: пересчитать и исправленное вручную")
    g.add_argument("-n", "--dry-run", action="store_true", help="только показать, что будет сделано")
    g.add_argument("--review", action="store_true", help="после обработки открыть страницу проверки")
    g.add_argument("-h", "--help", action="help", help="эта справка")

    a = p.parse_args(argv)
    if a.square:
        a.crop = True
    if a.fast:
        a.model = engine.FAST_MODEL
    if not 1 <= a.quality <= 100:
        p.error("--quality: от 1 до 100")
    if a.bg:
        try:
            if a.bg.lower() not in ("transparent", "none"):
                engine.parse_color(a.bg)
        except ValueError:
            p.error(f"--bg: не понимаю цвет «{a.bg}» (пример: \"#f8f1e7\", white)")
    if a.discard_edits and not a.overwrite:
        p.error("--discard-edits имеет смысл только вместе с --overwrite")
    return a


def cmd_run(argv: list[str]) -> int:
    a = parse_run(argv)

    if len(a.inputs) == 1 and is_result_dir(a.inputs[0]):
        print(f"Это папка результатов. Открыть её в окне программы: {bold('cutout review ' + a.inputs[0])}")
        return 2

    render = engine.Render(
        format=a.format, bg=a.bg, crop=a.crop, pad=a.pad, square=a.square,
        max_size=a.max_size, quality=a.quality,
        clean_edges=not a.raw_edges, specks=not a.keep_specks,
    )
    if render.format == "jpg" and not render.bg:
        print(warn("JPEG не бывает прозрачным — фон будет белым (другой цвет: --bg)."))
    try:
        batch = Batch(a.inputs, a.out, model=a.model, render=render, recursive=not a.no_subfolders,
                      overwrite=a.overwrite, discard_edits=a.discard_edits, want_mask=a.mask)
    except BatchError as e:
        print(bad(str(e)))
        return 2

    c = batch.counts
    print(f"{bold('Откуда:')} {short(batch.base)}")
    print(f"{bold('Куда:')}   {short(batch.out)}")
    print(f"{bold('Модель:')} {a.model} — {engine.MODELS[a.model]}")
    parts = [f"картинок {c['images']}"]
    for key, word in (("new", "вырезать"), ("render", "пересобрать"), ("skip", "уже готово"),
                      ("broken", "не открываются"), ("other", "не картинки")):
        if c[key]:
            parts.append(f"{word} {c[key]}")
    print(" · ".join(parts))
    if c["new"]:
        print(dim(f"Примерно {human_time(batch.estimate())} на этом Mac. Прервать можно в любой момент (Ctrl+C) — повторный запуск продолжит."))
    print()

    if a.dry_run:
        label = {"new": "вырезать", "render": "пересобрать", "skip": "готово", "broken": "битый"}
        for f, rel, out_rel, action, why in batch.jobs:
            print(f"  {label[action]:<11} {rel}  →  {out_rel}" + (dim(f"  ({why})") if why else ""))
        for s_ in batch.skipped:
            print(dim(f"  пропуск     {os.path.relpath(s_, batch.base)} — не картинка"))
        return 0

    try:
        lock = FolderLock(batch.out, "обработка в терминале").__enter__()
    except Busy as b:
        print(bad(f"Эта папка результатов сейчас занята: {b}."))
        print("Если она открыта в окне программы — закройте окно и подождите минуту, или работайте в окне.")
        return 2
    try:
        summary = batch.run(_printer(batch))
    finally:
        lock.__exit__(None, None, None)

    if summary["interrupted"]:
        return 130
    if a.review:
        return open_window(review=batch.out)
    return 1 if summary["errors"] else 0


def _printer(batch: Batch):
    width = len(str(batch.counts["new"] + batch.counts["render"]))

    def emit(kind: str, **d):
        if kind == "model_loading":
            print(dim("Загружаю модель (при самом первом запуске она скачивается, ≈1 ГБ)…"), flush=True)
        elif kind == "model_ready":
            print(dim(f"Модель готова за {human_time(d['seconds'])}."), flush=True)
        elif kind == "error":
            prefix = f"[{d['k']:>{width}}/{d['total']}] " if d["k"] else ""
            print(f"{prefix}{bad('ошибка')} {d['rel']}: {d['msg']}", flush=True)
        elif kind == "item":
            note = ("  " + warn("⚠ " + "; ".join(engine.FLAGS[x] for x in d["flags"]))) if d["flags"] else ""
            verb = "пересобрано" if d["action"] == "render" else f"{d['seconds']:4.1f} с"
            eta = f"  осталось ≈{human_time(d['eta'])}" if d["eta"] >= 1 and d["k"] < d["total"] else ""
            why = dim(f"  ({d['why']})") if d["why"] else ""
            print(f"[{d['k']:>{width}}/{d['total']}] {d['rel']} → {d['out']}  {dim(verb)}{note}{why}{dim(eta)}", flush=True)
        elif kind == "done":
            print()
            if d["interrupted"]:
                print(warn("Остановлено. Запустите ту же команду снова — продолжит с места."))
            head = f"Сделано до остановки: {d['done']}" if d["interrupted"] else f"Готово: {d['done']}"
            skip = batch.counts["skip"]
            print(bold(head) + dim(f" за {human_time(d['seconds'])}") + (f" · уже было готово: {skip}" if skip else ""))
            if d["errors"]:
                log = os.path.join(batch.store.meta, "errors.log")
                print(bad(f"Ошибок: {len(d['errors'])}") + (dim(f" (подробности: {short(log)})") if os.path.exists(log) else ""))
                for rel, msg in d["errors"][:10]:
                    print(bad(f"  {rel}: {msg}"))
            if d["flagged"]:
                print(warn(f"Стоит посмотреть глазами: {d['flagged']}") + dim(" (рука в кадре, коллаж, несколько частей)"))
            print(f"Проверить и поправить щелчком: {bold('cutout review ' + short(batch.out))}")

    return emit


def cmd_review(argv: list[str]) -> int:
    p = argparse.ArgumentParser(
        prog="cutout review",
        usage="cutout review ПАПКА_РЕЗУЛЬТАТОВ",
        description="Открыть папку результатов в окне программы: проверка и правка щелчком.",
        add_help=False,
    )
    p._positionals.title = "что открыть"
    p._optionals.title = "параметры"
    p.add_argument("folder", nargs="?", default=".", metavar="ПАПКА_РЕЗУЛЬТАТОВ",
                   help="папка, которую создал cutout (по умолчанию — текущая)")
    p.add_argument("--no-open", action="store_true", help="не открывать браузер самому")
    p.add_argument("-h", "--help", action="help", help="эта справка")
    a = p.parse_args(argv)

    folder = os.path.abspath(os.path.expanduser(a.folder))
    if not is_result_dir(folder):
        sibling = folder.rstrip(os.sep) + "-cutout"
        if is_result_dir(sibling):
            folder = sibling
        else:
            print(bad(f"В {short(folder)} нет результатов cutout."))
            return 2
    return open_window(review=folder, open_browser=not a.no_open)


def open_window(paths: list[str] | None = None, review: str | None = None, open_browser: bool = True) -> int:
    import app

    return app.launch(paths, review=review, open_browser=open_browser)


APP_NAME = "Вырезать фигурки"


def install() -> int:
    """Значок на рабочем столе: двойной щелчок открывает окно, брошенная на
    него папка или фото сразу подставляются в окно."""
    launcher = os.path.join(HERE, "cutout")
    log = os.path.expanduser("~/.cutout/server.log")
    q = launcher.replace('"', '\\"')
    script = f'''on run
	do shell script "mkdir -p ~/.cutout; \\"{q}\\" app > \\"{log}\\" 2>&1 &"
end run

on open theItems
	set args to ""
	repeat with i in theItems
		set args to args & " " & quoted form of POSIX path of i
	end repeat
	do shell script "mkdir -p ~/.cutout; \\"{q}\\" app" & args & " > \\"{log}\\" 2>&1 &"
end open
'''
    import subprocess
    import tempfile

    target = os.path.expanduser(f"~/Desktop/{APP_NAME}.app")
    with tempfile.NamedTemporaryFile("w", suffix=".applescript", delete=False, encoding="utf-8") as f:
        f.write(script)
        src = f.name
    if os.path.exists(target):
        subprocess.run(["rm", "-rf", target], check=True)
    r = subprocess.run(["osacompile", "-o", target, src], capture_output=True, text=True)
    os.remove(src)
    if r.returncode != 0:
        print(bad("Не удалось создать значок: " + r.stderr.strip()))
        return 1
    # Значок программы с папкой на входе — «капельница»: macOS берёт её
    # картинку из droplet.icns, а имя из CFBundleIconName ведёт в Assets.car,
    # который перебил бы свою иконку стандартной. Поэтому имя снимается.
    icon = os.path.join(HERE, "icon.icns")
    res = os.path.join(target, "Contents", "Resources")
    plist = os.path.join(target, "Contents", "Info.plist")
    if os.path.exists(icon):
        for name in ("droplet.icns", "applet.icns"):
            subprocess.run(["cp", icon, os.path.join(res, name)])
        subprocess.run(["/usr/libexec/PlistBuddy", "-c", "Delete :CFBundleIconName", plist], capture_output=True)
        try:
            os.remove(os.path.join(res, "Assets.car"))
        except OSError:
            pass
    subprocess.run(["/usr/libexec/PlistBuddy", "-c", "Add :CFBundleIdentifier string local.gotiga.cutout", plist],
                   capture_output=True)
    subprocess.run(["codesign", "--force", "--sign", "-", target], capture_output=True)  # правка пакета снимает подпись
    subprocess.run(["touch", target])
    print(f"Готово: на рабочем столе значок «{APP_NAME}».")
    print("Двойной щелчок — открыть программу; перетащите на него папку с фото — она сразу подставится.")
    return 0


def main(argv: list[str]) -> int:
    if not argv:
        return open_window()
    if argv[0] == "app":
        return open_window(argv[1:])
    if argv[0] == "review":
        return cmd_review(argv[1:])
    if argv[0] in ("--install", "install"):
        return install()
    if argv[0] == "help":
        parse_run(["--help"])
    return cmd_run(argv)


def finish(code: int) -> None:
    """Выход мимо уборки интерпретатора.

    К этому мигу все файлы и журнал уже записаны. Обычный выход разбирает
    объекты onnxruntime в произвольном порядке и изредка падает в C++
    («recursive_mutex lock failed»): результат цел, но терминал показывает
    аварию. Прямой выход этого шага не делает."""
    mp = sys.modules.get("multiprocessing.util")
    if mp is not None:  # семафоры полосы загрузки, иначе трекер ругается на утечку
        try:
            mp._run_finalizers(0)
        except Exception:
            pass
    sys.stdout.flush()
    sys.stderr.flush()
    os._exit(code)


def _as_interrupt(signum, frame):
    raise KeyboardInterrupt


if __name__ == "__main__":
    import signal

    # Закрытое окно терминала (SIGHUP) и остановка извне (SIGTERM) — то же,
    # что Ctrl+C: страница проверки дописывает правки, пакет снимает замок.
    for sig in (signal.SIGTERM, signal.SIGHUP):
        signal.signal(sig, _as_interrupt)
    try:
        code = main(sys.argv[1:])
    except KeyboardInterrupt:
        code = 130
    except SystemExit as ex:  # argparse: --help и ошибки разбора
        code = ex.code if isinstance(ex.code, int) else (0 if ex.code is None else 2)
    finish(code)
