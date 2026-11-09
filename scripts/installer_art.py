#!/usr/bin/env python3
"""Gera as imagens do assistente de instalação do Windows a partir da marca.

Lê os SVG de assets/brand (o camaleão e o nome "AURORA") e grava em
installer/windows/art/ os PNG que o aurora.iss usa:

  wizard-164.png, wizard-328.png   a faixa da esquerda das páginas de
                                   boas-vindas e de fim (100% e 200%), na
                                   proporção do WizardSizePercent do aurora.iss
  small-55.png, small-110.png      o ícone do canto das outras páginas

Só a biblioteca padrão: o desenho é feito aqui (preenchimento com a regra
nonzero, com antisserrilhado por amostragem) e o PNG é escrito à mão. Rode
de novo quando a marca mudar:

  python scripts/installer_art.py
"""

import math
import re
import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BRAND = ROOT / "assets" / "brand"
OUT = ROOT / "installer" / "windows" / "art"

# As cores do tema Boreal da AURORA IDE: o fundo grafite do ícone, o azul do
# camaleão no escuro e o texto claro.
GRAPHITE = (0x14, 0x16, 0x19)
BLUE = (0x59, 0xA0, 0xF9)
LIGHT = (0xEA, 0xED, 0xF1)

# O WizardSizePercent do aurora.iss (largura, altura): a faixa da esquerda,
# 164 por 314 no tamanho 100,100, cresce nessa proporção.
SIZE_PERCENT = (130, 150)


# ------------------------------------------------------------- SVG -----

def tokens(d):
    return re.findall(r"[MmLlHhVvCcQqZz]|-?(?:\d+\.?\d*|\.\d+)(?:e-?\d+)?", d)


def flatten(d, steps=12):
    """Os contornos de um caminho SVG (M, L, H, V, C, Q e Z, absolutos ou
    relativos) em polígonos; cada curva vira `steps` retas."""
    shapes, current = [], []
    x = y = sx = sy = 0.0
    toks = tokens(d)
    i, cmd = 0, None

    def num():
        nonlocal i
        i += 1
        return float(toks[i - 1])

    while i < len(toks):
        if toks[i].isalpha():
            cmd = toks[i]
            i += 1
            if cmd in "Zz":
                if current:
                    shapes.append(current)
                current = []
                x, y = sx, sy
                continue
        rel = cmd.islower()
        ox, oy = (x, y) if rel else (0.0, 0.0)
        c = cmd.upper()
        if c == "M":
            if current:
                shapes.append(current)
            x, y = ox + num(), oy + num()
            sx, sy = x, y
            current = [(x, y)]
            cmd = "l" if rel else "L"
        elif c == "L":
            x, y = ox + num(), oy + num()
            current.append((x, y))
        elif c == "H":
            x = ox + num()
            current.append((x, y))
        elif c == "V":
            y = oy + num()
            current.append((x, y))
        elif c == "C":
            c1 = (ox + num(), oy + num())
            c2 = (ox + num(), oy + num())
            end = (ox + num(), oy + num())
            for k in range(1, steps + 1):
                t = k / steps
                u = 1 - t
                current.append((
                    u**3 * x + 3 * u * u * t * c1[0] + 3 * u * t * t * c2[0] + t**3 * end[0],
                    u**3 * y + 3 * u * u * t * c1[1] + 3 * u * t * t * c2[1] + t**3 * end[1],
                ))
            x, y = end
        elif c == "Q":
            c1 = (ox + num(), oy + num())
            end = (ox + num(), oy + num())
            for k in range(1, steps + 1):
                t = k / steps
                u = 1 - t
                current.append((
                    u * u * x + 2 * u * t * c1[0] + t * t * end[0],
                    u * u * y + 2 * u * t * c1[1] + t * t * end[1],
                ))
            x, y = end
    if current:
        shapes.append(current)
    return shapes


def brand_shape(name):
    """O desenho de um SVG da marca: os polígonos de todos os caminhos e o
    viewBox (x, y, largura, altura)."""
    svg = (BRAND / name).read_text(encoding="utf-8")
    box = tuple(float(v) for v in re.search(r'viewBox="([^"]+)"', svg).group(1).split())
    polygons = []
    for d in re.findall(r' d="([^"]+)"', svg):
        polygons.extend(flatten(d))
    return polygons, box


# ---------------------------------------------------------- desenho -----

class Canvas:
    def __init__(self, w, h):
        self.w, self.h = w, h
        self.px = [[(0, 0, 0, 0)] * w for _ in range(h)]

    def fill_rect(self, color, alpha=1.0):
        for y in range(self.h):
            for x in range(self.w):
                self.blend(x, y, color, alpha)

    def blend(self, x, y, color, alpha):
        if alpha <= 0 or not (0 <= x < self.w and 0 <= y < self.h):
            return
        r, g, b, a = self.px[y][x]
        a0 = a / 255
        out_a = alpha + a0 * (1 - alpha)
        if out_a == 0:
            return
        mix = [
            (c * alpha + o * a0 * (1 - alpha)) / out_a
            for c, o in zip(color, (r, g, b))
        ]
        self.px[y][x] = (*[round(v) for v in mix], round(out_a * 255))

    def fill(self, polygons, color, alpha=1.0, samples=4):
        """Preenchimento nonzero, com `samples` x `samples` amostras por pixel."""
        edges = []
        for poly in polygons:
            for (x0, y0), (x1, y1) in zip(poly, poly[1:] + poly[:1]):
                if y0 != y1:
                    edges.append((x0, y0, x1, y1))
        if not edges:
            return
        ys = [e[1] for e in edges] + [e[3] for e in edges]
        top, bottom = max(0, int(min(ys))), min(self.h - 1, int(max(ys)) + 1)
        for py in range(top, bottom + 1):
            row = [0.0] * self.w
            for s in range(samples):
                sy = py + (s + 0.5) / samples
                hits = []
                for x0, y0, x1, y1 in edges:
                    if (y0 <= sy < y1) or (y1 <= sy < y0):
                        hits.append((x0 + (sy - y0) * (x1 - x0) / (y1 - y0), 1 if y1 > y0 else -1))
                hits.sort()
                wind = 0
                for (xa, da), (xb, _) in zip(hits, hits[1:]):
                    wind += da
                    if wind != 0:
                        for sx in range(int(xa * samples), int(xb * samples)):
                            px = sx // samples
                            if 0 <= px < self.w:
                                row[px] += 1 / (samples * samples)
            for px, c in enumerate(row):
                if c > 0:
                    self.blend(px, py, color, min(1.0, c) * alpha)

    def round_mask(self, radius):
        """Cantos arredondados: transparente fora do retângulo de cantos `radius`."""
        for y in range(self.h):
            for x in range(self.w):
                cx = min(max(x + 0.5, radius), self.w - radius)
                cy = min(max(y + 0.5, radius), self.h - radius)
                d = math.hypot(x + 0.5 - cx, y + 0.5 - cy)
                keep = max(0.0, min(1.0, radius + 0.5 - d))
                r, g, b, a = self.px[y][x]
                self.px[y][x] = (r, g, b, round(a * keep))

    def save(self, path):
        raw = b"".join(
            b"\x00" + bytes(v for p in row for v in p) for row in self.px
        )

        def chunk(kind, data):
            body = kind + data
            return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

        png = b"\x89PNG\r\n\x1a\n"
        png += chunk(b"IHDR", struct.pack(">IIBBBBB", self.w, self.h, 8, 6, 0, 0, 0))
        png += chunk(b"IDAT", zlib.compress(raw, 9))
        png += chunk(b"IEND", b"")
        path.write_bytes(png)


def place(polygons, box, width, left, top):
    """Os polígonos de um desenho com o viewBox `box`, na largura `width`,
    com o canto de cima à esquerda em (`left`, `top`)."""
    s = width / box[2]
    return [[((x - box[0]) * s + left, (y - box[1]) * s + top) for x, y in poly] for poly in polygons]


# ---------------------------------------------------------- imagens -----

def wizard(w, h):
    """A faixa da esquerda: fundo grafite, o camaleão azul e o nome embaixo
    dele, um pouco acima do meio da faixa."""
    mark, mark_box = brand_shape("aurora-mark.svg")
    word, word_box = brand_shape("aurora-wordmark.svg")
    k = w / 164
    c = Canvas(w, h)
    c.fill_rect(GRAPHITE)

    mark_w = 76 * k
    word_w = 112 * k
    word_h = word_w * word_box[3] / word_box[2]
    gap = 26 * k
    top = h * 0.42 - (mark_w + gap + word_h) / 2
    c.fill(place(mark, mark_box, mark_w, (w - mark_w) / 2, top), BLUE)
    c.fill(place(word, word_box, word_w, (w - word_w) / 2, top + mark_w + gap), LIGHT)
    return c


def small(size):
    """O ícone da AURORA IDE (aurora-icon.svg): quadrado grafite de cantos
    redondos com o camaleão azul."""
    mark, mark_box = brand_shape("aurora-mark.svg")
    c = Canvas(size, size)
    c.fill_rect(GRAPHITE)
    c.round_mask(size * 14 / 64)
    c.fill(place(mark, mark_box, size * 40 / 64, size * 12 / 64, size * 12 / 64), BLUE)
    return c


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for w in (164, 328):
        stretch = SIZE_PERCENT[1] / SIZE_PERCENT[0]
        wizard(w, round(w * 314 * stretch / 164)).save(OUT / f"wizard-{w}.png")
    for size in (55, 110):
        small(size).save(OUT / f"small-{size}.png")
    for p in sorted(OUT.glob("*.png")):
        print(f"{p.relative_to(ROOT)}  {p.stat().st_size} bytes")


if __name__ == "__main__":
    main()
