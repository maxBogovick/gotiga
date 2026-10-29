#!/usr/bin/env python3
"""Поиск по локальному индексу (`build.py`): слова, вопросы, фото.

    .venv-tools/embed/bin/python tools/embed/search.py "ведьма с метлой"
    ... search.py "где живёт гном" --kind tale          # только байки (work, tale, card, leaf)
    ... search.py "снежная зима" --photos               # фото по описанию словами
    ... search.py --like ~/Downloads/foto.jpg           # фото, похожие на эту картинку
    ... search.py --serve                               # страница http://127.0.0.1:8765

Находка — вещь целиком (работа, байка…), оценка — лучший из её кусков на любом из
шести языков: запрос по-русски находит английский текст. Перебор по всем векторам —
несколько миллисекунд на тысячи кусков, отдельная база векторов не нужна.
"""
import argparse
import io
import json
import sys
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlparse

import numpy as np

sys.path.insert(0, str(Path(__file__).parent))
from models import ImageModel, TextModel  # noqa: E402

HERE = Path(__file__).parent
INDEX = HERE / "index"
FALLBACK = {"ru": ["ru", "uk", "en"], "uk": ["uk", "ru", "en"], "en": ["en", "ru"],
            "es": ["es", "en", "ru"], "de": ["de", "en", "ru"], "it": ["it", "en", "ru"]}
PATH = {"work": "/figurines/", "tale": "/tales/", "leaf": "/gazette/", "card": "/battles?card="}


class Index:
    def __init__(self, folder=INDEX):
        self.meta = json.load(open(folder / "meta.json"))
        self.items = json.load(open(folder / "items.json", encoding="utf-8"))
        self.rows = json.load(open(folder / "text.json", encoding="utf-8"))
        self.tvec = np.load(folder / "text.npy").astype(np.float32)
        self.ids = np.array([r["id"] for r in self.rows])
        self.kinds = np.array([r["kind"] for r in self.rows])
        self.photos, self.ivec = [], None
        if self.meta.get("image_model"):
            self.photos = json.load(open(folder / "image.json", encoding="utf-8"))
            self.ivec = np.load(folder / "image.npy").astype(np.float32)
        self._tm = self._im = None

    @property
    def tm(self):
        self._tm = self._tm or TextModel(self.meta["text_model"])
        return self._tm

    @property
    def im(self):
        self._im = self._im or ImageModel(self.meta["image_model"])
        return self._im

    def title(self, item_id, lang="ru"):
        t = self.items[item_id]["title"]
        return next((t[l] for l in FALLBACK.get(lang, ["en"]) if t.get(l)), next(iter(t.values()), item_id))

    def href(self, item_id):
        it = self.items[item_id]
        return PATH[it["kind"]] + (it.get("slug") or item_id)

    def text(self, q, k=8, kind=None, lang="ru"):
        s = self.tvec @ self.tm.encode_queries([q])[0]
        if kind:
            s = np.where(self.kinds == kind, s, -1)
        best = {}
        for j in np.argsort(-s)[:k * 12]:
            i = self.ids[j]
            if i not in best:
                best[i] = j
            if len(best) >= k:
                break
        out = []
        for i, j in best.items():
            r = self.rows[j]
            out.append({"id": i, "kind": r["kind"], "title": self.title(i, lang), "score": float(s[j]),
                        "lang": r["lang"], "part": r["part"], "snippet": r["text"][:260],
                        "href": self.href(i)})
        return out

    def _photos(self, s, k, lang):
        out, seen = [], {}
        for j in np.argsort(-s):
            p = self.photos[j]
            seen[p["work"]] = seen.get(p["work"], 0) + 1
            if seen[p["work"]] > 2:  # не больше двух кадров одной работы подряд
                continue
            out.append({"id": p["image"], "work": p["work"], "title": self.title(p["work"], lang),
                        "score": float(s[j]), "href": self.href(p["work"])})
            if len(out) >= k:
                break
        return out

    def photos_by_text(self, q, k=12, lang="ru"):
        return self._photos(self.ivec @ self.im.encode_queries([q])[0], k, lang)

    def photos_like(self, source, k=12, lang="ru"):
        """`source` — путь к файлу, байты картинки или id фото из индекса."""
        if isinstance(source, str) and not Path(source).exists():
            v = self.ivec[[p["image"] for p in self.photos].index(source)]
        else:
            if isinstance(source, bytes):
                tmp = INDEX / "_query.jpg"
                tmp.write_bytes(source)
                source = tmp
            v = self.im.encode_images([Path(source)])[0]
        return self._photos(self.ivec @ v, k, lang)


def show(res, photos=False):
    for r in res:
        if photos:
            print(f"  {r['score']:.3f}  {r['title']}  [{r['id'][:8]}]")
        else:
            print(f"  {r['score']:.3f}  [{r['kind']}] {r['title']}  ({r['lang']}/{r['part']})")
            print("         " + r["snippet"].replace("\n", " ")[:150])


PAGE = """<!doctype html><meta charset=utf-8><title>Поиск по смыслу</title>
<meta name=viewport content="width=device-width,initial-scale=1">
<style>
body{margin:0;background:#f8f1e7;color:#34251c;font:16px/1.5 Georgia,serif}
main{max-width:1100px;margin:0 auto;padding:24px 16px 64px}
form{display:flex;gap:8px;margin-bottom:12px}
input[type=text]{flex:1;padding:12px 14px;font:inherit;border:1px solid #d8c6b1;background:#fffaf2;border-radius:8px}
button,.pick{padding:10px 16px;font:inherit;border:1px solid #6f3b24;background:#6f3b24;color:#f8f1e7;border-radius:8px;cursor:pointer}
.opts{display:flex;gap:16px;flex-wrap:wrap;margin-bottom:20px;font-size:14px;color:#5b4637;align-items:center}
h2{font-size:15px;letter-spacing:.06em;text-transform:uppercase;color:#6f3b24;margin:28px 0 10px}
.hit{padding:10px 0;border-bottom:1px solid #d8c6b1}
.hit b{font-weight:600}.hit small{color:#6a5646}.hit p{margin:4px 0 0;color:#4d3a2d;font-size:14px}
.score{font:12px ui-monospace,monospace;color:#8a6a4f}
.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(150px,1fr));gap:12px}
.ph{display:block;text-decoration:none;color:inherit;font-size:12px}
.ph img{width:100%;aspect-ratio:3/4;object-fit:cover;border-radius:8px;border:1px solid #d8c6b1;display:block;cursor:zoom-in}
#drop{border:1px dashed #b89c7e;border-radius:8px;padding:6px 12px;font-size:14px}
</style>
<main>
<form id=f><input type=text id=q placeholder="Ведьма с метлой · who lives in the forest · снежная зима" autofocus>
<button>Искать</button></form>
<div class=opts>
<label><input type=checkbox id=photos checked> фото</label>
<label>тип: <select id=kind><option value="">всё</option><option value=work>работы</option>
<option value=tale>байки</option><option value=card>карты</option><option value=leaf>вестник</option></select></label>
<span id=drop>перетащите сюда картинку — найду похожие фото</span><span id=t class=score></span></div>
<div id=out></div></main>
<script>
const $=id=>document.getElementById(id),esc=s=>s.replace(/[&<>]/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;'}[c]));
function draw(d){let h='';
 if(d.text&&d.text.length){h+='<h2>Тексты</h2>'+d.text.map(r=>`<div class=hit><b>${esc(r.title)}</b> <small>${r.kind} · ${r.lang}</small> <span class=score>${r.score.toFixed(3)}</span> <small>${esc(r.href)}</small><p>${esc(r.snippet)}…</p></div>`).join('')}
 if(d.photos&&d.photos.length){h+='<h2>Фото</h2><div class=grid>'+d.photos.map(p=>`<a class=ph href="/photo/${p.id}.jpg" target=_blank><img loading=lazy src="/photo/${p.id}.jpg"><div>${esc(p.title)} <span class=score>${p.score.toFixed(3)}</span></div></a>`).join('')+'</div>'}
 $('out').innerHTML=h||'<p>Ничего не нашлось.</p>';$('t').textContent=d.ms+' мс'}
async function go(e){e&&e.preventDefault();const q=$('q').value.trim();if(!q)return;
 const r=await fetch('/api/search?'+new URLSearchParams({q,kind:$('kind').value,photos:$('photos').checked?1:0}));draw(await r.json())}
$('f').onsubmit=go;$('kind').onchange=go;$('photos').onchange=go;
document.body.ondragover=e=>e.preventDefault();
document.body.ondrop=async e=>{e.preventDefault();const f=e.dataTransfer.files[0];if(!f)return;
 $('t').textContent='считаю…';const r=await fetch('/api/like',{method:'POST',body:f});const d=await r.json();draw({photos:d.photos,ms:d.ms})};
</script>"""


def serve(idx, port):
    class H(BaseHTTPRequestHandler):
        def log_message(self, *a):
            pass

        def send(self, body, ctype="application/json", code=200):
            body = body if isinstance(body, bytes) else body.encode()
            self.send_response(code)
            self.send_header("Content-Type", ctype)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self):
            u = urlparse(self.path)
            if u.path == "/":
                return self.send(PAGE, "text/html; charset=utf-8")
            if u.path.startswith("/photo/"):
                f = INDEX / "photos" / Path(u.path).name
                return self.send(f.read_bytes(), "image/jpeg") if f.exists() else self.send("", code=404)
            if u.path == "/api/search":
                a = {k: v[0] for k, v in parse_qs(u.query).items()}
                t0 = time.time()
                out = {"text": idx.text(a["q"], 10, a.get("kind") or None)}
                if a.get("photos") == "1" and idx.ivec is not None:
                    out["photos"] = idx.photos_by_text(a["q"], 12)
                out["ms"] = round((time.time() - t0) * 1000)
                return self.send(json.dumps(out, ensure_ascii=False))
            self.send("", code=404)

        def do_POST(self):
            if self.path != "/api/like":
                return self.send("", code=404)
            t0 = time.time()
            data = self.rfile.read(int(self.headers["Content-Length"]))
            out = {"photos": idx.photos_like(data, 12), "ms": round((time.time() - t0) * 1000)}
            self.send(json.dumps(out, ensure_ascii=False))

    ThreadingHTTPServer(("127.0.0.1", port), H).serve_forever()


def main():
    p = argparse.ArgumentParser()
    p.add_argument("query", nargs="?")
    p.add_argument("-k", type=int, default=8)
    p.add_argument("--kind", choices=["work", "tale", "card", "leaf"])
    p.add_argument("--photos", action="store_true", help="искать фото по описанию")
    p.add_argument("--like", help="картинка или id фото: найти похожие")
    p.add_argument("--serve", action="store_true")
    p.add_argument("--port", type=int, default=8765)
    a = p.parse_args()
    idx = Index()
    if a.serve:
        idx.tm, idx.im  # noqa: B018  — прогреть модели до первого запроса
        print(f"http://127.0.0.1:{a.port}  (Ctrl+C — выйти)")
        return serve(idx, a.port)
    t0 = time.time()
    if a.like:
        show(idx.photos_like(a.like, a.k), photos=True)
    elif a.query and a.photos:
        show(idx.photos_by_text(a.query, a.k), photos=True)
    elif a.query:
        show(idx.text(a.query, a.k, a.kind))
    else:
        p.error("нужен запрос, --like или --serve")
    print(f"  ({(time.time() - t0) * 1000:.0f} мс с загрузкой модели)")


if __name__ == "__main__":
    main()
