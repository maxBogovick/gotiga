"""Правка щелчком в одной папке результатов: маски, отмена, превью.

Правка меняет маску, а не картинку: маска лежит в `.cutout/edit/`, итоговый
файл пересобирается из исходника тем же `engine.compose`, что и в пакете.
Пересборка идёт в фоне — ответ на щелчок не ждёт записи PNG."""

from __future__ import annotations

import os
import threading
import time
from collections import OrderedDict

import numpy as np
from PIL import Image

import engine
from store import Store, now

UNDO_DEPTH = 30


class Desk:
    """Одна папка результатов на столе: журнал, отмена, фоновая сборка.

    Модель щелчков общая на все папки (`clicker`, `sam_lock`): грузится она
    секунд десять и весит сотни мегабайт."""

    def __init__(self, store: Store, clicker: engine.Clicker, sam_lock: threading.Lock):
        self.store = store
        self.clicker = clicker
        self.sam_lock = sam_lock                 # модель одна, щелчки по очереди
        self.edit_lock = threading.Lock()        # маски правятся по очереди
        self.undo: dict[str, list] = {}           # id -> [маска | None(=как у модели)]
        self.images: OrderedDict[str, tuple] = OrderedDict()
        self.img_lock = threading.Lock()
        self.pending: dict[str, float] = {}
        self.render_lock = threading.Lock()      # один файл не пишется двумя потоками
        self.cv = threading.Condition()
        self.worker = threading.Thread(target=self._render_loop, daemon=True)
        self.worker.start()

    # ── картинки ────────────────────────────────────────────────────────
    def image(self, iid: str) -> tuple[Image.Image, bytes | None]:
        with self.img_lock:
            if iid in self.images:
                self.images.move_to_end(iid)
                return self.images[iid]
        pair = engine.load_image(self.store.items[iid]["src"])
        with self.img_lock:
            self.images[iid] = pair
            while len(self.images) > 4:
                self.images.popitem(last=False)
        return pair

    def mask(self, iid: str) -> np.ndarray:
        return engine.load_mask(self.store.current_mask(iid))

    def version(self, iid: str) -> str:
        p = self.store.current_mask(iid)
        try:
            return str(int(os.stat(p).st_mtime_ns // 1_000_000))
        except OSError:
            return "0"

    def final_version(self, iid: str) -> str:
        """Версия готового файла. Правка сначала меняет маску, а файл
        собирается следом, — поэтому берётся позднее из двух: адрес плитки
        меняется сразу, а сервер, получив его, дособирает файл до ответа."""
        try:
            out = int(os.stat(self.store.out_path(iid)).st_mtime_ns // 1_000_000)
        except OSError:
            out = 0
        return str(max(out, int(self.version(iid))))

    def item_json(self, iid: str) -> dict:
        e = self.store.items[iid]
        return {
            "id": iid,
            "rel": e["rel"],
            "out": e["out"],
            "flags": e.get("flags", []),
            "edited": bool(e.get("edited")),
            "approved": bool(e.get("approved")),
            "size": e.get("size"),
            "v": self.version(iid),
            "fv": self.final_version(iid),
            "missing": not os.path.exists(e["src"]),
        }

    # ── правка ──────────────────────────────────────────────────────────
    def _key(self, iid: str) -> str:
        return f"{self.store.out}:{iid}:{self.store.items[iid].get('sig')}"

    def prepare(self, iid: str) -> None:
        img, _ = self.image(iid)
        with self.sam_lock:
            self.clicker.prepare(self._key(iid), img)

    def edit(self, iid: str, op: str, body: dict) -> dict:
        img, _ = self.image(iid)
        w, h = img.size
        with self.edit_lock:
            before = self.mask(iid)
            alpha = before.copy()
            if op in ("remove", "restore"):
                x = float(body["x"]) * w
                y = float(body["y"]) * h
                with self.sam_lock:
                    obj = self.clicker.select(self._key(iid), img, x, y)
                if op == "remove":
                    m = min(w, h)
                    # на пару пикселей шире предмета: иначе от руки остаётся обводка
                    alpha *= 1.0 - engine.grow(obj, max(2, round(m * 0.003)))
                    # Дальше по соседству гасится только полупрозрачное — тень
                    # контура снятой руки. Плотное (пальцы фигурки у точки
                    # касания) остаётся. Глобально так чистить нельзя: далеко
                    # от тела лежат и настоящие выбившиеся пряди.
                    near = engine.grow(obj, max(6, round(m * 0.012))) > 0.5
                    alpha[near & (alpha < 0.5)] = 0.0
                else:
                    region = obj > 0.5
                    auto = engine.load_mask(self.store.auto_mask(iid))
                    # если модель этот предмет видела — возвращаем её мягкий край,
                    # иначе берём край SAM, чуть смягчённый
                    if region.any() and float(auto[region].mean()) > 0.3:
                        add = auto * engine.grow(obj, max(2, round(min(w, h) * 0.004)))
                    else:
                        add = engine.grow(obj, 1)
                    alpha = np.maximum(alpha, add)
            elif op in ("erase", "keep"):
                x0, x1 = sorted((float(body["x0"]), float(body["x1"])))
                y0, y1 = sorted((float(body["y0"]), float(body["y1"])))
                X0, X1 = int(round(x0 * w)), int(round(x1 * w))
                Y0, Y1 = int(round(y0 * h)), int(round(y1 * h))
                if op == "erase":
                    alpha[max(0, Y0):Y1, max(0, X0):X1] = 0.0
                else:
                    keep = np.zeros_like(alpha)
                    keep[max(0, Y0):Y1, max(0, X0):X1] = alpha[max(0, Y0):Y1, max(0, X0):X1]
                    alpha = keep
            else:
                raise ValueError(op)
            self._push(iid)
            engine.save_mask(alpha, self.store.edit_mask(iid))
            self.store.update(iid, edited=True, approved=False, edited_at=now())
        self._after_change(iid, img, alpha)
        return self.item_json(iid)

    def _push(self, iid: str) -> None:
        stack = self.undo.setdefault(iid, [])
        e = self.store.edit_mask(iid)
        stack.append(engine.load_mask(e) if os.path.exists(e) else None)
        del stack[:-UNDO_DEPTH]

    def undo_last(self, iid: str) -> dict:
        with self.edit_lock:
            stack = self.undo.get(iid) or []
            if not stack:
                return self.item_json(iid)
            prev = stack.pop()
            self._set(iid, prev)
        return self.item_json(iid)

    def reset(self, iid: str) -> dict:
        with self.edit_lock:
            if os.path.exists(self.store.edit_mask(iid)):
                self._push(iid)
                self._set(iid, None)
        return self.item_json(iid)

    def _set(self, iid: str, mask: np.ndarray | None) -> None:
        e = self.store.edit_mask(iid)
        if mask is None:
            if os.path.exists(e):
                os.remove(e)
            self.store.update(iid, edited=False, approved=False)
        else:
            engine.save_mask(mask, e)
            self.store.update(iid, edited=True, approved=False)
        img, _ = self.image(iid)
        self._after_change(iid, img, self.mask(iid))

    def approve(self, iid: str, value: bool) -> dict:
        self.store.update(iid, approved=bool(value))
        return self.item_json(iid)

    def _after_change(self, iid: str, img: Image.Image, alpha: np.ndarray) -> None:
        r = engine.Render.from_dict(self.store.items[iid].get("render", {}))
        flags, stats = engine.analyze(img, engine.final_alpha(alpha, r))
        self.store.update(iid, flags=flags, stats=stats)
        self.store.drop_thumbs(iid)
        with self.cv:
            self.pending[iid] = time.time()
            self.cv.notify()

    # ── фоновая сборка итоговых файлов ───────────────────────────────────
    def _render_loop(self) -> None:
        while True:
            with self.cv:
                while not self.pending:
                    self.cv.wait()
                # подождать, пока щелчки по этой картинке утихнут
                iid, t = min(self.pending.items(), key=lambda kv: kv[1])
                wait = t + 0.6 - time.time()
                if wait > 0:
                    self.cv.wait(wait)
                    continue
                del self.pending[iid]
            try:
                self.render_final(iid)
            except Exception as ex:  # pragma: no cover
                print(f"не удалось собрать {self.store.items[iid]['out']}: {ex}", flush=True)

    def render_final(self, iid: str) -> None:
        with self.render_lock:
            e = self.store.items[iid]
            r = engine.Render.from_dict(e.get("render", {}))
            img, icc = self.image(iid)
            out = engine.compose(img, self.mask(iid), r)
            engine.save(out, os.path.join(self.store.out, e["out"]), r, icc)

    def flush_one(self, iid: str) -> None:
        with self.cv:
            waiting = self.pending.pop(iid, None) is not None
        if waiting:
            self.render_final(iid)

    def flush(self) -> None:
        with self.cv:
            ids = list(self.pending)
            self.pending.clear()
        for iid in ids:
            self.render_final(iid)

    # ── превью ──────────────────────────────────────────────────────────
    def _final_preview(self, iid: str, width: int) -> bytes:
        """Уменьшенный готовый файл — то, что получится на самом деле:
        с обрезкой, фоном и форматом."""
        self.flush_one(iid)
        v = self.final_version(iid)
        cache = os.path.join(self.store.thumbs(), f"{iid}-final-{width}-{v}.webp")
        if os.path.exists(cache):
            with open(cache, "rb") as f:
                return f.read()
        with Image.open(self.store.out_path(iid)) as im:
            im.load()
            im = im.convert("RGBA")
        data = engine.thumb_bytes(im, width, quality=85)
        os.makedirs(os.path.dirname(cache), exist_ok=True)
        with open(cache + ".part", "wb") as f:
            f.write(data)
        os.replace(cache + ".part", cache)
        return data

    def preview(self, iid: str, kind: str, width: int) -> bytes:
        width = max(64, min(width or 1600, 2400))
        if kind == "final":
            return self._final_preview(iid, width)
        v = self.version(iid)
        cache = os.path.join(self.store.thumbs(), f"{iid}-{kind}-{width}-{v}.webp")
        if os.path.exists(cache):
            with open(cache, "rb") as f:
                return f.read()
        img, _ = self.image(iid)
        k = min(1.0, width / img.width)
        size = (max(1, round(img.width * k)), max(1, round(img.height * k)))
        small = img.resize(size, Image.Resampling.LANCZOS) if k < 1 else img
        if kind == "src":
            data = engine.thumb_bytes(small, 0, quality=82)
        else:
            r = engine.Render.from_dict(self.store.items[iid].get("render", {}))
            a = self.mask(iid)
            if r.specks:
                a = engine.drop_specks(a)
            am = Image.fromarray(np.clip(a * 255 + 0.5, 0, 255).astype(np.uint8), "L").resize(size, Image.Resampling.LANCZOS)
            af = np.asarray(am, dtype=np.float32) / 255.0
            # на мелкой сетке очистка края не видна, а стоит времени
            rgb = engine.foreground(small, af) if (r.clean_edges and width >= 900) else np.asarray(small)
            rgba = Image.fromarray(np.dstack([rgb, np.asarray(am)]), "RGBA")
            data = engine.thumb_bytes(rgba, 0, quality=88)
        os.makedirs(os.path.dirname(cache), exist_ok=True)
        with open(cache + ".part", "wb") as f:
            f.write(data)
        os.replace(cache + ".part", cache)
        return data
