"""Иконки и заставки приложения из static/images/raven-emblem.png.

Запуск из корня проекта: python3 tools/pwa/make_icons.py
Пишет в static/pwa/ и обновляет apple-touch-icon.png (непрозрачный: iOS заливает
прозрачность чёрным). Заставки iOS — пергамент и медаль по центру.
"""
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "static" / "pwa"
PAPER = (248, 241, 231, 255)  # --color-canvas, #f8f1e7

emblem = Image.open(ROOT / "static/images/raven-emblem.png").convert("RGBA")


def on_paper(size, share):
    """Медаль на сплошном пергаменте; share — доля стороны, занятая медалью."""
    canvas = Image.new("RGBA", (size, size), PAPER)
    d = round(size * share)
    m = emblem.resize((d, d), Image.LANCZOS)
    canvas.alpha_composite(m, ((size - d) // 2, (size - d) // 2))
    return canvas


def save(img, name):
    img.convert("RGBA").save(OUT / name, optimize=True)


# «any»: медаль на прозрачном фоне, почти во всю сторону (как была на сайте).
for s in (192, 512):
    m = emblem.resize((s, s), Image.LANCZOS)
    save(m, f"icon-{s}.png")

# maskable: безопасная зона — круг в 80 % стороны; медаль 70 % вписана с запасом.
for s in (192, 512):
    save(on_paper(s, 0.70), f"icon-maskable-{s}.png")

# monochrome (тематические значки Android): силуэт птицы и каймы одним цветом.
mono_src = emblem.resize((512, 512), Image.LANCZOS)
px = mono_src.load()
mono = Image.new("RGBA", mono_src.size, (0, 0, 0, 0))
mp = mono.load()
for y in range(mono_src.height):
    for x in range(mono_src.width):
        r, g, b, a = px[x, y]
        lum = 0.299 * r + 0.587 * g + 0.114 * b
        if a > 0 and lum < 150:
            mp[x, y] = (0, 0, 0, min(255, a))
canvas = Image.new("RGBA", (512, 512), (0, 0, 0, 0))
inner = mono.resize((358, 358), Image.LANCZOS)
canvas.alpha_composite(inner, ((512 - 358) // 2, (512 - 358) // 2))
save(canvas, "icon-monochrome-512.png")

# iOS: значок на экран «Домой» — непрозрачный.
on_paper(180, 0.86).convert("RGB").save(ROOT / "static/apple-touch-icon.png", optimize=True)
on_paper(180, 0.86).convert("RGB").save(ROOT / "static/apple-touch-icon-precomposed.png", optimize=True)

# Заставки iOS: (ширина, высота) в пикселях устройства, (ширина, высота, плотность) в точках.
SPLASH = [
    (1320, 2868, 440, 956, 3), (1206, 2622, 402, 874, 3), (1290, 2796, 430, 932, 3),
    (1179, 2556, 393, 852, 3), (1170, 2532, 390, 844, 3), (1125, 2436, 375, 812, 3),
    (1242, 2688, 414, 896, 3), (828, 1792, 414, 896, 2), (1242, 2208, 414, 736, 3),
    (750, 1334, 375, 667, 2), (640, 1136, 320, 568, 2),
    (2048, 2732, 1024, 1366, 2), (1668, 2388, 834, 1194, 2), (1640, 2360, 820, 1180, 2),
    (1668, 2224, 834, 1112, 2), (1536, 2048, 768, 1024, 2),
]
(OUT / "splash").mkdir(exist_ok=True)
links = []
for w, h, cw, ch, ratio in SPLASH:
    img = Image.new("RGB", (w, h), PAPER[:3])
    d = round(min(w, h) * 0.34)
    m = emblem.resize((d, d), Image.LANCZOS)
    img.paste(m, ((w - d) // 2, (h - d) // 2 - round(h * 0.03)), m)
    name = f"splash/{w}x{h}.png"
    img.save(OUT / name, optimize=True)
    links.append(
        f'<link rel="apple-touch-startup-image" href="/pwa/{name}" '
        f'media="(device-width: {cw}px) and (device-height: {ch}px) and '
        f'(-webkit-device-pixel-ratio: {ratio}) and (orientation: portrait)" />'
    )
(ROOT / "tools/pwa/splash-links.html").write_text("\n".join(links) + "\n")
print("готово:", len(SPLASH), "заставок")
