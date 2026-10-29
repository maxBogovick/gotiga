"""Манифест приложения на шести языках: static/site.webmanifest (en) и
static/pwa/manifest.<язык>.webmanifest. Слова разделов — те же, что в шапке
(`navAllWorks`, `navTales`, `navGame`, `navGazette`).

Запуск из корня проекта: python3 tools/pwa/make_manifests.py
Скриншоты для окна установки берутся из static/pwa/screenshots/ (если есть).
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "static" / "pwa"

TEXT = {
    "en": dict(name="Ritunia — Cabinet of Gothic Miniatures",
               description="An author's cabinet of gothic, handmade miniatures and art dolls.",
               works="All works", tales="Tall tales", game="Game", gazette="Gazette"),
    "ru": dict(name="Ritunia — готические миниатюры",
               description="Авторские готические миниатюры и куклы ручной работы.",
               works="Все работы", tales="Небылицы", game="Игра", gazette="Вестник"),
    "uk": dict(name="Ritunia — готичні мініатюри",
               description="Авторські готичні мініатюри та ляльки ручної роботи.",
               works="Усі роботи", tales="Небилиці", game="Гра", gazette="Вісник"),
    "es": dict(name="Ritunia — miniaturas góticas",
               description="Miniaturas góticas y muñecas de arte hechas a mano por su autora.",
               works="Todas las obras", tales="Cuentos", game="Juego", gazette="Gaceta"),
    "de": dict(name="Ritunia — gotische Miniaturen",
               description="Gotische Miniaturen und Kunstpuppen, von der Autorin von Hand gefertigt.",
               works="Alle Werke", tales="Geschichten", game="Spiel", gazette="Zeitung"),
    "it": dict(name="Ritunia — miniature gotiche",
               description="Miniature gotiche e bambole d'arte fatte a mano dall'autrice.",
               works="Tutte le opere", tales="Racconti", game="Gioco", gazette="Gazzetta"),
}

PAPER = "#f8f1e7"


def screenshots():
    shots = []
    folder = OUT / "screenshots"
    if not folder.exists():
        return shots
    from PIL import Image
    for f in sorted(folder.glob("*.png")) + sorted(folder.glob("*.webp")):
        w, h = Image.open(f).size
        shots.append({
            "src": f"/pwa/screenshots/{f.name}",
            "sizes": f"{w}x{h}",
            "type": "image/webp" if f.suffix == ".webp" else "image/png",
            "form_factor": "wide" if w > h else "narrow",
        })
    return shots


def manifest(lang):
    s = TEXT[lang]
    m = {
        "id": "/",
        "name": s["name"],
        "short_name": "Ritunia",
        "description": s["description"],
        "lang": lang,
        "dir": "ltr",
        "start_url": "/?src=pwa",
        "scope": "/",
        "display": "standalone",
        "orientation": "any",
        "background_color": PAPER,
        "theme_color": PAPER,
        "categories": ["art", "entertainment", "lifestyle"],
        "launch_handler": {"client_mode": "navigate-existing"},
        "prefer_related_applications": False,
        "icons": [
            {"src": "/pwa/icon-192.png", "sizes": "192x192", "type": "image/png", "purpose": "any"},
            {"src": "/pwa/icon-512.png", "sizes": "512x512", "type": "image/png", "purpose": "any"},
            {"src": "/pwa/icon-maskable-192.png", "sizes": "192x192", "type": "image/png", "purpose": "maskable"},
            {"src": "/pwa/icon-maskable-512.png", "sizes": "512x512", "type": "image/png", "purpose": "maskable"},
            {"src": "/pwa/icon-monochrome-512.png", "sizes": "512x512", "type": "image/png", "purpose": "monochrome"},
        ],
        "shortcuts": [
            {"name": s["works"], "url": "/figurines?src=pwa_shortcut"},
            {"name": s["tales"], "url": "/tales?src=pwa_shortcut"},
            {"name": s["game"], "url": "/battles/etude?src=pwa_shortcut"},
            {"name": s["gazette"], "url": "/gazette?src=pwa_shortcut"},
        ],
    }
    shots = screenshots()
    if shots:
        m["screenshots"] = shots
    return m


def write(path, data):
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n")


for lang in TEXT:
    write(OUT / f"manifest.{lang}.webmanifest", manifest(lang))
write(ROOT / "static" / "site.webmanifest", manifest("en"))
print("манифестов:", len(TEXT))
