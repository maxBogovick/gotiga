"""Папка результатов и её журнал.

Рядом с готовыми файлами лежит скрытая `.cutout/`:

    .cutout/manifest.json   что из чего сделано, какой моделью, с какими настройками
    .cutout/auto/<id>.png   маска, которую дала модель
    .cutout/edit/<id>.png   маска после правок щелчком (если правили)
    .cutout/thumbs/         уменьшенные копии для страницы проверки

Маски хранятся затем, чтобы смена фона, обрезки или формата не запускала
нейросеть заново, а правка щелчком переживала повторный запуск."""

from __future__ import annotations

import hashlib
import json
import os
import threading
import time

META = ".cutout"


def item_id(rel_src: str) -> str:
    return hashlib.sha1(rel_src.encode("utf-8")).hexdigest()[:16]


class Store:
    def __init__(self, out_dir: str):
        self.out = os.path.abspath(out_dir)
        self.meta = os.path.join(self.out, META)
        self.path = os.path.join(self.meta, "manifest.json")
        self.lock = threading.RLock()
        self.data = {"version": 1, "items": {}}
        if os.path.exists(self.path):
            with open(self.path, encoding="utf-8") as f:
                self.data = json.load(f)
            self.data.setdefault("items", {})

    @staticmethod
    def exists(out_dir: str) -> bool:
        return os.path.exists(os.path.join(out_dir, META, "manifest.json"))

    @property
    def items(self) -> dict:
        return self.data["items"]

    def auto_mask(self, iid: str) -> str:
        return os.path.join(self.meta, "auto", iid + ".png")

    def edit_mask(self, iid: str) -> str:
        return os.path.join(self.meta, "edit", iid + ".png")

    def current_mask(self, iid: str) -> str:
        e = self.edit_mask(iid)
        return e if os.path.exists(e) else self.auto_mask(iid)

    def thumbs(self) -> str:
        return os.path.join(self.meta, "thumbs")

    def out_path(self, iid: str) -> str:
        return os.path.join(self.out, self.items[iid]["out"])

    def put(self, iid: str, entry: dict) -> None:
        with self.lock:
            self.items[iid] = entry
            self.save()

    def update(self, iid: str, **fields) -> dict:
        with self.lock:
            self.items[iid].update(fields)
            self.save()
            return self.items[iid]

    def save(self) -> None:
        """Журнал пишется после каждого файла и атомарно: прерванный на
        пятисотом фото пакет продолжается с пятьсот первого."""
        with self.lock:
            os.makedirs(self.meta, exist_ok=True)
            tmp = self.path + ".part"
            with open(tmp, "w", encoding="utf-8") as f:
                json.dump(self.data, f, ensure_ascii=False, indent=1)
            os.replace(tmp, self.path)

    def drop_thumbs(self, iid: str) -> None:
        d = self.thumbs()
        if not os.path.isdir(d):
            return
        for name in os.listdir(d):
            if name.startswith(iid):
                try:
                    os.remove(os.path.join(d, name))
                except OSError:
                    pass


class Busy(Exception):
    """Папкой уже занят другой запуск cutout."""


class FolderLock:
    """Одна папка — один процесс.

    Пакет и страница проверки держат журнал в памяти и пишут его целиком;
    работая разом над одной папкой, они затирали бы записи друг друга.
    Замок — файл с номером процесса; замок умершего процесса не держит."""

    def __init__(self, out_dir: str, what: str):
        self.path = os.path.join(out_dir, META, "lock")
        self.what = what

    def __enter__(self):
        os.makedirs(os.path.dirname(self.path), exist_ok=True)
        try:
            with open(self.path, encoding="utf-8") as f:
                pid_s, _, what = f.read().partition(" ")
            pid = int(pid_s)
            if pid != os.getpid() and _alive(pid):
                raise Busy(f"{what.strip() or 'другой запуск'} (процесс {pid})")
        except (OSError, ValueError):
            pass
        with open(self.path, "w", encoding="utf-8") as f:
            f.write(f"{os.getpid()} {self.what}")
        return self

    def __exit__(self, *exc):
        try:
            with open(self.path, encoding="utf-8") as f:
                if f.read().split(" ")[0] == str(os.getpid()):
                    os.remove(self.path)
        except OSError:
            pass
        return False


def _alive(pid: int) -> bool:
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


def now() -> str:
    return time.strftime("%Y-%m-%d %H:%M:%S")
