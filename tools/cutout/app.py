"""Окно программы: локальный сервер на 127.0.0.1 и страница `app.html`.

Всё делается на одной странице: выбрать фото (кнопкой, перетаскиванием или
на значок программы), настроить, запустить, смотреть ход, поправить щелчком,
открыть результат в Finder. Терминал для этого не нужен.

Сервер один на компьютер (`~/.cutout/server.json`): второй запуск не
поднимает второй сервер, а открывает окно первого. Выключается он сам —
когда страница закрыта десять минут и работы нет. Обработка, начатая на
странице, не останавливается от закрытой вкладки: открыли снова — видно,
где она."""

from __future__ import annotations

import hashlib
import json
import os
import re
import socket
import subprocess
import sys
import threading
import time
import traceback
import urllib.request
import webbrowser
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, unquote, urlparse

import engine
from batch import Batch, BatchError, collect, default_out, human_time, is_result_dir, short
from desk import Desk
from store import Busy, FolderLock, Store

HERE = os.path.dirname(os.path.abspath(__file__))
# Переменные окружения — для проверок: стенд не трогает настоящие «Недавние».
HOME_DIR = os.environ.get("CUTOUT_HOME") or os.path.expanduser("~/.cutout")
SERVER_FILE = os.path.join(HOME_DIR, "server.json")
SETTINGS_FILE = os.path.join(HOME_DIR, "settings.json")
RECENT_FILE = os.path.join(HOME_DIR, "recent.json")
DROPS = os.environ.get("CUTOUT_DROPS") or os.path.expanduser("~/Pictures/Вырезанные фигурки")
IDLE_EXIT = 600  # секунд без страницы и без работы — и сервер выключается

# Настройки окна → Render. Слова, а не флаги: страница показывает их кнопками.
DEFAULT_SETTINGS = {
    "bg": "none",          # none | #f8f1e7 | #ffffff | любой цвет
    "format": "png",       # png | webp | jpg
    "crop": "none",        # none | fit | square
    "max_size": 0,         # 0 | 2000 | 1200 | 800
    "model": engine.DEFAULT_MODEL,
}


def to_render(s: dict) -> tuple[engine.Render, str]:
    bg = s.get("bg") or "none"
    fmt = s.get("format") if s.get("format") in ("png", "webp", "jpg") else "png"
    crop = s.get("crop") or "none"
    model = s.get("model") if s.get("model") in engine.MODELS else engine.DEFAULT_MODEL
    try:
        size = max(0, int(s.get("max_size") or 0))
    except (TypeError, ValueError):
        size = 0
    if bg != "none":
        engine.parse_color(bg)  # ValueError — неверный цвет
    r = engine.Render(format=fmt, bg=None if bg == "none" else bg,
                      crop=crop in ("fit", "square"), square=crop == "square", max_size=size)
    return r, model


def settings_of(store: Store) -> dict | None:
    """Настройки окна по тому, как собраны файлы папки (для папок из терминала)."""
    for e in store.items.values():
        r = e.get("render") or {}
        crop = "square" if r.get("square") else ("fit" if r.get("crop") else "none")
        return {"bg": r.get("bg") or "none", "format": r.get("format", "png"), "crop": crop,
                "max_size": r.get("max_size", 0), "model": e.get("model", engine.DEFAULT_MODEL)}
    return None


def fid_of(out: str) -> str:
    return hashlib.sha1(out.encode("utf-8")).hexdigest()[:12]


def read_json(path: str, default):
    try:
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    except (OSError, ValueError):
        return default


def write_json(path: str, data) -> None:
    os.makedirs(os.path.dirname(path), exist_ok=True)
    tmp = path + ".part"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=1)
    os.replace(tmp, path)


def notify(title: str, text: str) -> None:
    """Уведомление macOS: обработка идёт десятки минут, и окно в это время
    обычно закрыто или спрятано."""
    script = f'display notification {json.dumps(text)} with title {json.dumps(title)}'
    try:
        subprocess.Popen(["osascript", "-e", script], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    except OSError:
        pass


def pick_dialog(kind: str) -> list[str] | None:
    """Обычное окно выбора macOS. None — нажали «Отменить»."""
    if kind == "files":
        script = '''activate
set fs to choose file with prompt "Выберите фото фигурок" of type {"public.image"} with multiple selections allowed
set out to ""
repeat with f in fs
  set out to out & POSIX path of f & linefeed
end repeat
return out'''
    elif kind == "out":
        script = 'activate\nreturn POSIX path of (choose folder with prompt "Куда класть вырезанные фигурки")'
    else:
        script = 'activate\nreturn POSIX path of (choose folder with prompt "Выберите папку с фото фигурок")'
    p = subprocess.run(["osascript", "-e", script], capture_output=True, text=True)
    if p.returncode != 0:
        return None  # -128: «Отменить»
    return [line.rstrip("/") or "/" for line in p.stdout.splitlines() if line.strip()]


# ── задание ──────────────────────────────────────────────────────────────

class Job:
    """Обработка, запущенная со страницы: идёт в фоновом потоке, страница
    спрашивает её состояние раз в секунду."""

    def __init__(self, batch: Batch, settings: dict, app: "App"):
        self.batch = batch
        self.settings = settings
        self.app = app
        self.out = batch.out
        self.fid = fid_of(batch.out)
        self.stop = threading.Event()
        self.state = "starting"   # starting | loading | running | done | stopped | failed
        self.total = batch.counts["new"] + batch.counts["render"]
        self.k = 0
        self.done = 0
        self.current = ""
        self.eta = batch.estimate()
        self.errors: list[list[str]] = []
        self.flagged = 0
        self.started = time.time()
        self.finished = 0.0
        self.rev = 0
        self.message = ""
        self.thread = threading.Thread(target=self._run, daemon=True)

    def _emit(self, kind: str, **d):
        if kind == "model_loading":
            self.state = "loading"
        elif kind == "model_ready":
            self.state = "running"
        elif kind == "start":
            self.state = "running"
            self.current = d["rel"]
        elif kind == "item":
            self.k, self.done = d["k"], self.done + 1
            self.eta = d["eta"]
            self.rev += 1
        elif kind == "error":
            self.errors.append([d["rel"], d["msg"]])
            if d.get("k"):
                self.k = d["k"]
            self.rev += 1
        elif kind == "done":
            self.flagged = d["flagged"]

    def _run(self):
        try:
            self.state = "running" if not self.batch.counts["new"] else "starting"
            s = self.batch.run(self._emit, self.stop)
            self.state = "stopped" if s["interrupted"] else "done"
        except Exception as ex:  # pragma: no cover
            self.state = "failed"
            self.message = f"{type(ex).__name__}: {ex}"
            traceback.print_exc()
        self.finished = time.time()
        self.current = ""
        self.rev += 1
        self.app.remember(self.out, self.batch.inputs, self.settings)
        if self.state == "done" and self.total:
            took = human_time(self.finished - self.started)
            extra = f", посмотреть: {self.flagged}" if self.flagged else ""
            notify("Фигурки вырезаны", f"{self.done} фото за {took}{extra}")

    def status(self) -> dict:
        return {
            "fid": self.fid,
            "out": self.out,
            "outShort": short(self.out),
            "name": os.path.basename(self.out),
            "state": self.state,
            "total": self.total,
            "k": self.k,
            "done": self.done,
            "current": self.current,
            "eta": round(self.eta),
            "etaText": human_time(self.eta) if self.eta >= 1 else "",
            "errors": self.errors[-20:],
            "errorCount": len(self.errors),
            "flagged": self.flagged,
            "elapsed": human_time((self.finished or time.time()) - self.started),
            "rev": self.rev,
            "message": self.message,
            "skipped": self.batch.counts["skip"],
        }


# ── программа ────────────────────────────────────────────────────────────

class App:
    def __init__(self):
        self.lock = threading.RLock()
        self.desks: dict[str, Desk] = {}       # fid -> Desk
        self.locks: dict[str, FolderLock] = {}
        self.clicker = engine.Clicker()
        self.sam_lock = threading.Lock()
        self.cutters: dict[str, engine.Cutter] = {}
        self.job: Job | None = None
        self.seen = time.time()
        self.drops: dict[str, str] = {}

    # ── папки ───────────────────────────────────────────────────────────
    def desk(self, out: str) -> Desk:
        """Папка результата на столе. Замок держится до выключения: пока окно
        работает с папкой, обработка в терминале её не тронет."""
        out = os.path.abspath(out)
        fid = fid_of(out)
        with self.lock:
            if fid in self.desks:
                return self.desks[fid]
            lock = FolderLock(out, "окно программы")
            lock.__enter__()  # Busy — пусть летит наружу
            self.locks[fid] = lock
            d = Desk(Store(out), self.clicker, self.sam_lock)
            self.desks[fid] = d
            return d

    def desk_by_fid(self, fid: str) -> Desk | None:
        with self.lock:
            if fid in self.desks:
                return self.desks[fid]
        for r in self.recent():
            if fid_of(r["out"]) == fid and is_result_dir(r["out"]):
                return self.desk(r["out"])
        return None

    def cutter(self, model: str) -> engine.Cutter:
        with self.lock:
            if model not in self.cutters:
                self.cutters = {model: engine.Cutter(model)}  # одна модель в памяти
            return self.cutters[model]

    # ── память между запусками ──────────────────────────────────────────
    def settings(self) -> dict:
        s = dict(DEFAULT_SETTINGS)
        s.update({k: v for k, v in read_json(SETTINGS_FILE, {}).items() if k in DEFAULT_SETTINGS})
        return s

    def recent(self) -> list[dict]:
        items = read_json(RECENT_FILE, [])
        return [r for r in items if isinstance(r, dict) and r.get("out")]

    def remember(self, out: str, inputs: list[str], settings: dict) -> None:
        with self.lock:
            rows = [r for r in self.recent() if r["out"] != out]
            rows.insert(0, {"out": out, "inputs": inputs, "settings": settings, "when": time.strftime("%Y-%m-%d %H:%M")})
            write_json(RECENT_FILE, rows[:30])
            write_json(SETTINGS_FILE, settings)

    def forget(self, out: str) -> None:
        with self.lock:
            write_json(RECENT_FILE, [r for r in self.recent() if r["out"] != out])

    def recent_view(self) -> list[dict]:
        rows = []
        for r in self.recent():
            out = r["out"]
            m = read_json(os.path.join(out, ".cutout", "manifest.json"), None)
            if not m:
                continue  # папку удалили или переместили
            items = m.get("items", {}).values()
            rows.append({
                "fid": fid_of(out),
                "out": out,
                "outShort": short(out),
                "name": os.path.basename(out),
                "inputs": [short(p) for p in r.get("inputs", [])],
                "when": r.get("when", ""),
                "count": len(items),
                "flagged": sum(1 for e in items if e.get("flags") and not e.get("approved")),
                "approved": sum(1 for e in items if e.get("approved")),
                "edited": sum(1 for e in items if e.get("edited")),
                "settings": r.get("settings") or {},
            })
        return rows

    # ── разбор и запуск ─────────────────────────────────────────────────
    def inspect(self, paths: list[str], out: str | None, settings: dict) -> dict:
        render, model = to_render(settings)
        files, skipped, base, single = collect(paths)
        if not files:
            raise BatchError("Здесь нет фото" + (f" — только другие файлы ({len(skipped)})." if skipped else "."))
        out = os.path.abspath(os.path.expanduser(out)) if out else default_out(base, single)
        fid = fid_of(out)
        store = self.desks[fid].store if fid in self.desks else Store(out)
        b = Batch(paths, out, model=model, render=render, store=store, cutter=self.cutters.get(model))
        return {
            "paths": [short(p) for p in paths],
            "base": short(b.base),
            "out": out,
            "outShort": short(out),
            "counts": b.counts,
            "estimate": round(b.estimate()),
            "estimateText": human_time(b.estimate()) if b.estimate() >= 1 else "",
        }

    def start(self, paths: list[str], out: str | None, settings: dict, overwrite: bool = False) -> Job:
        with self.lock:
            if self.job and self.job.thread.is_alive():
                raise BatchError("Уже идёт обработка — дождитесь её или остановите.")
            render, model = to_render(settings)
            files, _, base, single = collect(paths)
            if not files:
                raise BatchError("Здесь нет фото.")
            out = os.path.abspath(os.path.expanduser(out)) if out else default_out(base, single)
            try:
                desk = self.desk(out)
            except Busy as b:
                raise BatchError(f"Эта папка результатов сейчас занята: {b}.")
            batch = Batch(paths, out, model=model, render=render, store=desk.store,
                          cutter=self.cutter(model), overwrite=overwrite)
            self.cutters = {model: batch.cutter}
            self.remember(out, batch.inputs, settings)
            self.job = Job(batch, settings, self)
            self.job.thread.start()
            return self.job

    # ── перетащенные в окно файлы ───────────────────────────────────────
    def drop_dir(self, drop: str) -> str:
        """Папка для перетащенного: браузер не сообщает, откуда файл, только
        его содержимое, — поэтому копия ложится в «Изображения»."""
        if not re.fullmatch(r"[A-Za-z0-9_-]{6,40}", drop):
            raise ValueError("drop")
        with self.lock:
            if drop not in self.drops:
                stamp = time.strftime("%Y-%m-%d %H-%M-%S")
                self.drops[drop] = os.path.join(DROPS, f"Фото {stamp}")
            return self.drops[drop]

    def save_upload(self, drop: str, rel: str, body: bytes) -> str:
        root = self.drop_dir(drop)
        rel = rel.replace("\\", "/").lstrip("/")
        parts = [p for p in rel.split("/") if p not in ("", ".")]
        if not parts or any(p == ".." for p in parts):
            raise ValueError("путь")
        path = os.path.join(root, *parts)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path + ".part", "wb") as f:
            f.write(body)
        os.replace(path + ".part", path)
        return root

    def drop_selection(self, drop: str) -> list[str]:
        """Что выбрано перетаскиванием: одна перетащенная папка — она сама
        (результат ляжет рядом «<папка>-cutout»), иначе — вся папка загрузки."""
        root = self.drop_dir(drop)
        entries = [e for e in os.listdir(root) if not e.startswith(".")] if os.path.isdir(root) else []
        if len(entries) == 1 and os.path.isdir(os.path.join(root, entries[0])):
            return [os.path.join(root, entries[0])]
        return [root]

    def flush(self) -> None:
        for d in list(self.desks.values()):
            try:
                d.flush()
                d.store.save()
            except Exception:
                traceback.print_exc()
        for lock in self.locks.values():
            lock.__exit__(None, None, None)


# ── HTTP ─────────────────────────────────────────────────────────────────

def make_handler(app: App):
    class H(BaseHTTPRequestHandler):
        server_version = "cutout"
        protocol_version = "HTTP/1.1"

        def log_message(self, fmt, *args):
            pass

        def _send(self, code: int, body: bytes, ctype: str, cache: bool = False):
            self.send_response(code)
            self.send_header("Content-Type", ctype)
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Cache-Control", "max-age=31536000, immutable" if cache else "no-store")
            self.end_headers()
            self.wfile.write(body)

        def _json(self, obj, code: int = 200):
            self._send(code, json.dumps(obj, ensure_ascii=False).encode("utf-8"), "application/json; charset=utf-8")

        def _fail(self, msg: str, code: int = 400):
            self._json({"error": msg}, code)

        def _desk(self, q) -> Desk | None:
            d = app.desk_by_fid((q.get("f") or [""])[0])
            if d is None:
                self._fail("папка не открыта — откройте её заново со стартовой страницы", 404)
            return d

        def _body(self) -> bytes:
            n = int(self.headers.get("Content-Length") or 0)
            return self.rfile.read(n) if n else b""

        def do_GET(self):
            app.seen = time.time()
            u = urlparse(self.path)
            parts = [unquote(p) for p in u.path.split("/") if p]
            q = parse_qs(u.query)
            try:
                if not parts:
                    with open(os.path.join(HERE, "app.html"), "rb") as f:
                        return self._send(200, f.read(), "text/html; charset=utf-8")
                if parts == ["api", "ping"]:
                    return self._json({"ok": True, "pid": os.getpid()})
                if parts == ["api", "home"]:
                    return self._json({
                        "settings": app.settings(),
                        "recent": app.recent_view(),
                        "job": app.job.status() if app.job else None,
                        "models": [{"key": k, "text": v, "seconds": engine.SECONDS[k]} for k, v in engine.MODELS.items()],
                        "drops": short(DROPS),
                    })
                if parts == ["api", "job"]:
                    return self._json(app.job.status() if app.job else None)
                if parts == ["api", "state"]:
                    d = self._desk(q)
                    if not d:
                        return
                    with d.store.lock:
                        ids = sorted(d.store.items, key=lambda i: d.store.items[i]["rel"].lower())
                    rec = next((r for r in app.recent() if r["out"] == d.store.out), {})
                    inputs = d.store.data.get("inputs") or rec.get("inputs") or []
                    return self._json({
                        "fid": fid_of(d.store.out),
                        "folder": d.store.out,
                        "folderShort": short(d.store.out),
                        "name": os.path.basename(d.store.out),
                        "items": [d.item_json(i) for i in ids],
                        "flagText": engine.FLAGS,
                        "samReady": app.clicker._sess is not None,
                        "inputs": inputs,
                        "inputsShort": [short(p) for p in inputs],
                        "inputsMissing": [short(p) for p in inputs if not os.path.exists(p)],
                        "settings": rec.get("settings") or settings_of(d.store) or app.settings(),
                    })
                if len(parts) == 3 and parts[0] == "img" and parts[1] in ("src", "cut", "final"):
                    d = self._desk(q)
                    if not d:
                        return
                    if parts[2] not in d.store.items:
                        return self._fail("нет такой картинки", 404)
                    w = int((q.get("w") or ["1600"])[0])
                    return self._send(200, d.preview(parts[2], parts[1], w), "image/webp", cache=True)
                if len(parts) == 3 and parts[0] == "file" and parts[1] == "final":
                    d = self._desk(q)
                    if not d:
                        return
                    if parts[2] not in d.store.items:
                        return self._fail("нет такой картинки", 404)
                    d.flush()
                    path = d.store.out_path(parts[2])
                    ext = os.path.splitext(path)[1].lower()
                    ctype = {".png": "image/png", ".webp": "image/webp", ".jpg": "image/jpeg"}.get(ext, "application/octet-stream")
                    with open(path, "rb") as f:
                        return self._send(200, f.read(), ctype)
                return self._fail("нет такого адреса", 404)
            except (BrokenPipeError, ConnectionResetError):
                pass
            except Exception as ex:
                traceback.print_exc()
                return self._fail(f"{type(ex).__name__}: {ex}", 500)

        def do_PUT(self):
            app.seen = time.time()
            u = urlparse(self.path)
            q = parse_qs(u.query)
            try:
                if u.path == "/api/upload":
                    body = self._body()
                    app.save_upload((q.get("drop") or [""])[0], (q.get("path") or [""])[0], body)
                    return self._json({"ok": True})
                return self._fail("нет такого адреса", 404)
            except ValueError as ex:
                return self._fail(f"неверная загрузка: {ex}")
            except OSError as ex:
                return self._fail(f"не удалось сохранить файл: {ex}", 500)

        def do_POST(self):
            app.seen = time.time()
            u = urlparse(self.path)
            parts = [unquote(p) for p in u.path.split("/") if p]
            q = parse_qs(u.query)
            raw = self._body()
            try:
                body = json.loads(raw or b"{}")
            except json.JSONDecodeError:
                return self._fail("неверный запрос")
            try:
                if parts == ["api", "pick"]:
                    got = pick_dialog(body.get("kind", "folder"))
                    return self._json({"paths": got} if got is not None else {"cancelled": True})
                if parts == ["api", "drop-done"]:
                    return self._json({"paths": app.drop_selection(body.get("drop", ""))})
                if parts == ["api", "inspect"]:
                    return self._json(app.inspect(body.get("paths") or [], body.get("out"), body.get("settings") or {}))
                if parts == ["api", "start"]:
                    job = app.start(body.get("paths") or [], body.get("out"), body.get("settings") or {},
                                    bool(body.get("overwrite")))
                    return self._json(job.status())
                if parts == ["api", "stop"]:
                    if app.job:
                        app.job.stop.set()
                    return self._json(app.job.status() if app.job else None)
                if parts == ["api", "forget"]:
                    app.forget(body.get("out", ""))
                    return self._json({"ok": True})
                if parts == ["api", "reveal"]:
                    path = body.get("path") or ""
                    d = app.desk_by_fid(body.get("f", "")) if body.get("f") else None
                    if d is not None:
                        path = d.store.out_path(body["id"]) if body.get("id") in d.store.items else d.store.out
                    if path and os.path.exists(path):
                        subprocess.Popen(["open", "-R", path] if os.path.isfile(path) else ["open", path])
                    return self._json({"ok": True})
                if len(parts) == 4 and parts[:2] == ["api", "item"]:
                    d = self._desk(q)
                    if not d:
                        return
                    iid, act = parts[2], parts[3]
                    if iid not in d.store.items:
                        return self._fail("нет такой картинки", 404)
                    if act == "prepare":
                        d.prepare(iid)
                        return self._json({"ok": True})
                    if act == "edit":
                        return self._json(d.edit(iid, body.get("op", ""), body))
                    if act == "undo":
                        return self._json(d.undo_last(iid))
                    if act == "reset":
                        return self._json(d.reset(iid))
                    if act == "approve":
                        return self._json(d.approve(iid, body.get("value", True)))
                return self._fail("нет такого действия", 404)
            except BatchError as ex:
                return self._fail(str(ex))
            except ValueError as ex:
                return self._fail(f"неверное значение: {ex}")
            except Busy as ex:
                return self._fail(f"папка занята: {ex}")
            except (BrokenPipeError, ConnectionResetError):
                pass
            except Exception as ex:
                traceback.print_exc()
                return self._fail(f"{type(ex).__name__}: {ex}", 500)

    return H


# ── запуск ───────────────────────────────────────────────────────────────

def running_server() -> int | None:
    """Порт уже работающего сервера, если он жив."""
    info = read_json(SERVER_FILE, None)
    if not info:
        return None
    try:
        with urllib.request.urlopen(f"http://127.0.0.1:{info['port']}/api/ping", timeout=2) as r:
            if json.load(r).get("ok"):
                return int(info["port"])
    except Exception:
        pass
    return None


def free_port(start: int = 8765) -> int:
    for port in range(start, start + 50):
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            if s.connect_ex(("127.0.0.1", port)) != 0:
                return port
    raise RuntimeError("нет свободного порта")


def launch(paths: list[str] | None = None, review: str | None = None, open_browser: bool = True,
           port: int = 0) -> int:
    """Открыть окно программы. Если сервер уже работает — открыть его
    страницу, второй не поднимать. Выбранное (папка, брошенная на значок)
    едет в адресе страницы: так его получит ровно та вкладка, что откроется."""
    paths = [os.path.abspath(os.path.expanduser(p)) for p in (paths or [])]
    open_browser = open_browser and not os.environ.get("CUTOUT_NO_BROWSER")
    hash_ = ""
    if paths:
        hash_ = "#paths=" + urllib.request.quote(json.dumps(paths, ensure_ascii=False))
    if review:
        review = os.path.abspath(os.path.expanduser(review))
        hash_ = "#open=" + urllib.request.quote(review)

    alive = running_server()
    if alive:
        url = f"http://127.0.0.1:{alive}/{hash_}"
        if review:  # папку надо внести в недавние, чтобы окно её узнало
            _remember_review(review)
        print(f"Окно программы уже работает: {url}")
        if open_browser:
            webbrowser.open(url)
        return 0

    app = App()
    if review:
        try:
            app.desk(review)
        except Busy as b:
            print(f"Эта папка результатов сейчас занята: {b}.")
            return 2
        _remember_review(review)

    port = port or int(os.environ.get("CUTOUT_PORT") or 0) or free_port()
    httpd = ThreadingHTTPServer(("127.0.0.1", port), make_handler(app))
    httpd.daemon_threads = True
    write_json(SERVER_FILE, {"port": port, "pid": os.getpid()})
    url = f"http://127.0.0.1:{port}/{hash_}"
    print(f"Окно программы: {url}")
    print("Выключится само через 10 минут после того, как окно закрыто и работы нет.")
    print("Выключить сейчас — Ctrl+C.", flush=True)
    if open_browser:
        threading.Timer(0.3, lambda: webbrowser.open(url)).start()

    def watchdog():
        while True:
            time.sleep(15)
            busy = app.job is not None and app.job.thread.is_alive()
            if not busy and time.time() - app.seen > IDLE_EXIT:
                print("Окно давно закрыто, работы нет — выключаюсь.", flush=True)
                httpd.shutdown()
                return

    threading.Thread(target=watchdog, daemon=True).start()
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        if app.job and app.job.thread.is_alive():
            print("Останавливаю обработку (готовое сохранено, продолжится с места)…", flush=True)
            app.job.stop.set()
            app.job.thread.join(timeout=60)
        httpd.server_close()
        app.flush()
        info = read_json(SERVER_FILE, {})
        if info.get("pid") == os.getpid():
            try:
                os.remove(SERVER_FILE)
            except OSError:
                pass
        print("Выключено.", flush=True)
    return 0


def _remember_review(out: str) -> None:
    rows = read_json(RECENT_FILE, [])
    prev = next((r for r in rows if isinstance(r, dict) and r.get("out") == out), None)
    rows = [r for r in rows if isinstance(r, dict) and r.get("out") != out]
    rows.insert(0, prev or {"out": out, "inputs": [], "settings": {}, "when": time.strftime("%Y-%m-%d %H:%M")})
    write_json(RECENT_FILE, rows[:30])


if __name__ == "__main__":  # pragma: no cover
    sys.exit(launch(sys.argv[1:]))
