"""Пакетная обработка: что вырезать, куда класть, сам проход по файлам.

Один и тот же проход запускают и окно программы (`app.py`, в фоновом потоке),
и командная строка (`cutout.py`, в терминале). О ходе работы он сообщает
событиями, а как их показать — решает тот, кто запустил: окно рисует полосу,
терминал печатает строки."""

from __future__ import annotations

import os
import threading
import time
import traceback
from typing import Callable

import engine
from store import Store, item_id, now


class BatchError(Exception):
    """Пакет нельзя начать — причина словами, для человека."""


def human_time(s: float) -> str:
    s = int(round(s))
    if s < 60:
        return f"{s} с"
    if s < 3600:
        return f"{s // 60} мин {s % 60:02d} с"
    return f"{s // 3600} ч {s % 3600 // 60:02d} мин"


def short(path: str) -> str:
    home = os.path.expanduser("~")
    return "~" + path[len(home):] if path.startswith(home + os.sep) else path


def readable(path: str) -> bool:
    """Открывается ли файл как картинка — по заголовку, без чтения пикселей."""
    from PIL import Image

    try:
        with Image.open(path) as im:
            im.size
        return True
    except Exception:
        return False


def explain(ex: Exception) -> str:
    """Ошибка словами, а не именем класса."""
    from PIL import UnidentifiedImageError

    if isinstance(ex, UnidentifiedImageError):
        return "не открывается как картинка (повреждён или это не картинка)"
    if isinstance(ex, PermissionError):
        return "нет прав на чтение или запись"
    if isinstance(ex, OSError) and ex.errno == 28:
        return "закончилось место на диске"
    if isinstance(ex, MemoryError):
        return "не хватило памяти — слишком большая картинка"
    return f"{type(ex).__name__}: {ex}"


def is_result_dir(path: str) -> bool:
    return os.path.isdir(path) and Store.exists(path)


# ── что обрабатывать ─────────────────────────────────────────────────────

def collect(inputs: list[str], recursive: bool = True) -> tuple[list[str], list[str], str, bool]:
    """Файлы картинок, пропущенные (не картинки), общая основа путей и
    «вход — ровно одна папка».

    Основа — общая папка всех входов: от неё считаются подпапки результата,
    поэтому структура архива переезжает в результат как была."""
    missing = [p for p in inputs if not os.path.exists(os.path.expanduser(p))]
    if missing:
        raise BatchError("Не найдено: " + ", ".join(missing))

    files: list[str] = []
    skipped: list[str] = []
    roots: list[str] = []
    only_dirs = True
    for raw in inputs:
        p = os.path.abspath(os.path.expanduser(raw))
        if os.path.isdir(p):
            roots.append(p)
            for dirpath, dirnames, filenames in os.walk(p):
                # результаты прошлых запусков и скрытые папки не берём
                dirnames[:] = sorted(
                    d for d in dirnames
                    if not d.startswith(".") and not Store.exists(os.path.join(dirpath, d))
                )
                for name in sorted(filenames):
                    if name.startswith("."):
                        continue  # ._файлы macOS, .DS_Store
                    full = os.path.join(dirpath, name)
                    (files if os.path.splitext(name)[1].lower() in engine.INPUT_EXTS else skipped).append(full)
                if not recursive:
                    dirnames[:] = []
        else:
            only_dirs = False
            roots.append(os.path.dirname(p))
            if os.path.splitext(p)[1].lower() in engine.INPUT_EXTS:
                files.append(p)
            else:
                skipped.append(p)

    base = os.path.commonpath(roots) if roots else os.getcwd()
    seen, uniq = set(), []
    for f in files:  # один и тот же файл, названный дважды, — один раз
        if f not in seen:
            seen.add(f)
            uniq.append(f)
    return uniq, skipped, base, only_dirs and len(roots) == 1


def default_out(base: str, single_dir: bool) -> str:
    """Папка результата по умолчанию: рядом с папкой, «<папка>-cutout»;
    для отдельных файлов — подпапка `cutout` рядом с ними."""
    return base.rstrip(os.sep) + "-cutout" if single_dir else os.path.join(base, "cutout")


def source_dirs(inputs: list[str]) -> set[str]:
    out = set()
    for p in inputs:
        p = os.path.abspath(os.path.expanduser(p))
        out.add(p if os.path.isdir(p) else os.path.dirname(p))
    return out


def out_names(files: list[str], base: str, ext: str) -> dict[str, str]:
    """Имя результата: тот же путь, новое расширение.

    `кот.jpg` и `кот.png` в одной папке дали бы один `кот.png`, и второй
    молча затёр бы первый, — такие получают имя с прежним расширением."""
    by_stem: dict[str, int] = {}
    for f in files:
        k = os.path.splitext(os.path.relpath(f, base))[0].lower()
        by_stem[k] = by_stem.get(k, 0) + 1
    names = {}
    for f in files:
        stem, src_ext = os.path.splitext(os.path.relpath(f, base))
        if by_stem[stem.lower()] > 1:
            stem = f"{stem}-{src_ext.lstrip('.').lower()}"
        names[f] = stem + ext
    return names


# ── один файл ────────────────────────────────────────────────────────────

def plan(store: Store, src: str, rel: str, out_rel: str, model: str, render: engine.Render,
         overwrite: bool, discard_edits: bool) -> tuple[str, str]:
    """Что делать с файлом: (действие, пояснение).

    new — посчитать маску моделью; render — пересобрать файл из сохранённой
    маски (сменились фон, обрезка, формат); skip — уже готово."""
    iid = item_id(rel)
    e = store.items.get(iid)
    st = os.stat(src)
    sig = [st.st_size, int(st.st_mtime)]
    if not e or not os.path.exists(store.auto_mask(iid)):
        return "new", ""
    if e.get("sig") != sig:
        return "new", "исходник изменился" + (" — ручные правки сброшены" if e.get("edited") else "")
    ready = (e.get("render") == render.as_dict() and e.get("out") == out_rel
             and os.path.exists(os.path.join(store.out, out_rel)))
    if e.get("edited") and not discard_edits:
        return ("skip", "исправлено вручную") if ready else ("render", "исправлено вручную, правки сохранены")
    if overwrite or e.get("model") != model:
        return "new", "другая модель" if e.get("model") != model else "заново"
    return ("skip", "") if ready else ("render", "новые настройки")


def run_one(store: Store, cutter: engine.Cutter, src: str, rel: str, out_rel: str,
            render: engine.Render, action: str, want_mask: bool = False) -> dict:
    iid = item_id(rel)
    img, icc = engine.load_image(src)
    prev = store.items.get(iid, {})
    t0 = time.time()
    if action == "new":
        alpha = cutter.mask(img)
        engine.save_mask(alpha, store.auto_mask(iid))
        if os.path.exists(store.edit_mask(iid)):
            os.remove(store.edit_mask(iid))
        edited, approved = False, False
    else:
        alpha = engine.load_mask(store.current_mask(iid))
        edited, approved = prev.get("edited", False), prev.get("approved", False)
    seconds = time.time() - t0

    final = engine.final_alpha(alpha, render)
    flags, stats = engine.analyze(img, final)
    out = engine.compose(img, final, render, cleaned=True)
    engine.save(out, os.path.join(store.out, out_rel), render, icc)

    old = prev.get("out")
    if old and old != out_rel:  # сменился формат: прежний файл — наш, убираем
        try:
            os.remove(os.path.join(store.out, old))
        except OSError:
            pass
    if want_mask:
        engine.save_mask(alpha, os.path.join(store.out, "masks", os.path.splitext(out_rel)[0] + ".png"))

    st = os.stat(src)
    entry = {
        "src": src,
        "rel": rel,
        "out": out_rel,
        "sig": [st.st_size, int(st.st_mtime)],
        "size": list(img.size),
        "model": cutter.model if action == "new" else prev.get("model", cutter.model),
        "render": render.as_dict(),
        "flags": flags,
        "stats": stats,
        "seconds": round(seconds, 2) if action == "new" else prev.get("seconds"),
        "edited": edited,
        "approved": approved,
        "done_at": now(),
    }
    store.put(iid, entry)
    store.drop_thumbs(iid)
    return entry


# ── пакет ────────────────────────────────────────────────────────────────

Emit = Callable[..., None]


class Batch:
    """Один пакет: разбор входов при создании, проход — `run()`.

    Созданный пакет уже знает всё, что сделает (`jobs`, `counts`), и ничего
    не записал: окно показывает это до нажатия «Начать», а `--dry-run`
    печатает и выходит."""

    def __init__(self, inputs: list[str], out_dir: str | None = None, *,
                 model: str = engine.DEFAULT_MODEL, render: engine.Render | None = None,
                 recursive: bool = True, overwrite: bool = False, discard_edits: bool = False,
                 want_mask: bool = False, store: Store | None = None,
                 cutter: engine.Cutter | None = None):
        self.inputs = list(inputs)
        self.model = model
        self.render = render or engine.Render()
        self.want_mask = want_mask
        files, self.skipped, self.base, single = collect(self.inputs, recursive)
        if not files:
            raise BatchError("Картинок не найдено" + (f" (не картинок: {len(self.skipped)})" if self.skipped else "") + ".")
        self.out = os.path.abspath(os.path.expanduser(out_dir)) if out_dir else default_out(self.base, single)
        # Папка результата не может быть папкой исходников: результат лёг бы
        # рядом с оригиналами (а то и поверх одноимённого PNG), и следующий
        # запуск принял бы вырезанное за новые фото.
        if self.out in source_dirs(self.inputs):
            raise BatchError(f"Папка результата совпадает с папкой исходников ({short(self.out)}) — выберите другую.")
        self.store = store or Store(self.out)
        self.cutter = cutter if cutter and cutter.model == model else engine.Cutter(model)

        names = out_names(files, self.base, self.render.ext())
        self.jobs: list[tuple[str, str, str, str, str]] = []
        for f in files:
            rel = os.path.relpath(f, self.base)
            action, why = plan(self.store, f, rel, names[f], model, self.render, overwrite, discard_edits)
            if action == "new" and not readable(f):
                # битый файл узнаётся до загрузки модели: иначе каждый
                # повторный запуск грузил бы её ради одного отказа
                action, why = "broken", "не открывается как картинка (повреждён или это не картинка)"
            self.jobs.append((f, rel, names[f], action, why))
        self.counts = {k: sum(1 for j in self.jobs if j[3] == k) for k in ("new", "render", "skip", "broken")}
        self.counts["images"] = len(files)
        self.counts["other"] = len(self.skipped)

    def estimate(self) -> float:
        return self.counts["new"] * engine.SECONDS[self.model] + self.counts["render"] * 0.6

    def run(self, emit: Emit, stop: threading.Event | None = None) -> dict:
        """Проход по файлам. События: model_loading, model_ready, item, error, done."""
        stop = stop or threading.Event()
        # Откуда сделана папка — в её же журнал: окно по нему пересобирает
        # с другими настройками и папку, сделанную в терминале.
        with self.store.lock:
            self.store.data["inputs"] = self.inputs
            self.store.data["base"] = self.base
            self.store.save()
        errors = [(j[1], j[4]) for j in self.jobs if j[3] == "broken"]
        for rel, msg in errors:
            emit("error", rel=rel, msg=msg, k=0, total=0)
        todo = [j for j in self.jobs if j[3] in ("new", "render")]
        started = time.time()
        done = 0
        done_new, spent_new = 0, 0.0
        interrupted = False
        try:
            if self.counts["new"]:
                emit("model_loading")
                t = time.time()
                self.cutter._load()
                emit("model_ready", seconds=time.time() - t)
            for k, (f, rel, out_rel, action, why) in enumerate(todo, 1):
                if stop.is_set():
                    interrupted = True
                    break
                t = time.time()
                emit("start", k=k, total=len(todo), rel=rel)
                try:
                    e = run_one(self.store, self.cutter, f, rel, out_rel, self.render, action, self.want_mask)
                except KeyboardInterrupt:
                    raise
                except Exception as ex:  # один битый файл не останавливает пакет
                    msg = explain(ex)
                    errors.append((rel, msg))
                    os.makedirs(self.store.meta, exist_ok=True)
                    with open(os.path.join(self.store.meta, "errors.log"), "a", encoding="utf-8") as log:
                        log.write(f"{now()} {f}\n{traceback.format_exc()}\n")
                    emit("error", k=k, total=len(todo), rel=rel, msg=msg)
                    continue
                dt = time.time() - t
                done += 1
                if action == "new":
                    done_new += 1
                    spent_new += dt
                left_new = sum(1 for j in todo[k:] if j[3] == "new")
                left_render = sum(1 for j in todo[k:] if j[3] == "render")
                per = spent_new / done_new if done_new else engine.SECONDS[self.model]
                eta = left_new * per + left_render * 0.6
                emit("item", k=k, total=len(todo), rel=rel, out=out_rel, action=action, why=why,
                     seconds=dt, flags=e["flags"], eta=eta, id=item_id(rel))
        except KeyboardInterrupt:
            interrupted = True
        self.store.save()
        summary = {
            "done": done,
            "errors": errors,
            "interrupted": interrupted,
            "seconds": time.time() - started,
            "flagged": sum(1 for it in self.store.items.values() if it.get("flags") and not it.get("approved")),
        }
        emit("done", **summary)
        return summary
