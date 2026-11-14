#!/usr/bin/env python3
"""Generates all pixel-art sprites for the 2D game into assets/sprites/.

Original art made for this project — released under CC0 1.0.
Run:  python3 crates/game/tools/make_sprites.py
Shapes are drawn with flat colors, then a 1px dark outline is added automatically.
"""
import math
import os
import random
from PIL import Image, ImageDraw

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
OUT_DIR = os.path.join(ROOT, "assets", "sprites")
os.makedirs(OUT_DIR, exist_ok=True)

OUT = (27, 33, 64, 255)
WHITE = (242, 244, 250, 255)
LIGHT = (205, 212, 228, 255)
MID = (152, 162, 188, 255)
DARK = (88, 98, 128, 255)
BLUE = (70, 120, 235, 255)
BLUE_L = (130, 180, 255, 255)
GLASS = (120, 200, 255, 255)
ORANGE = (255, 140, 50, 255)
GREEN = (80, 200, 110, 255)
GREEN_D = (40, 140, 70, 255)
RED = (235, 70, 70, 255)
YELLOW = (255, 210, 70, 255)
CYAN = (90, 240, 220, 255)
T = (0, 0, 0, 0)


def new(w, h):
    return Image.new("RGBA", (w, h), T)


def outline(img, color=OUT):
    """Adds a 1px outline around opaque pixels (4-neighbourhood)."""
    w, h = img.size
    src = img.load()
    res = img.copy()
    dst = res.load()
    for y in range(h):
        for x in range(w):
            if src[x, y][3] != 0:
                continue
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                nx, ny = x + dx, y + dy
                if 0 <= nx < w and 0 <= ny < h and src[nx, ny][3] > 128:
                    dst[x, y] = color
                    break
    return res


def save(img, name):
    img.save(os.path.join(OUT_DIR, name))


# ------------------------------------------------------------------ astronaut

def astro_frame(direction, frame):
    """direction: 0 down, 1 left, 2 right, 3 up. Returns (body, visor) 16x24 images."""
    body = new(16, 24)
    visor = new(16, 24)
    d = ImageDraw.Draw(body)
    v = ImageDraw.Draw(visor)
    step = [0, 1, 0, -1][frame]
    bob = 1 if frame in (1, 3) else 0
    y0 = 1 + bob
    # legs
    if direction in (0, 3):
        d.rectangle((4, 18 + y0 - 1, 6, 21 + y0 - 1 + max(0, step)), fill=LIGHT)
        d.rectangle((9, 18 + y0 - 1, 11, 21 + y0 - 1 + max(0, -step)), fill=LIGHT)
        d.rectangle((4, 21 + y0 - 1 + max(0, step), 6, 22 + y0 - 1 + max(0, step)), fill=DARK)
        d.rectangle((9, 21 + y0 - 1 + max(0, -step), 11, 22 + y0 - 1 + max(0, -step)), fill=DARK)
    else:
        d.rectangle((6 + step, 18 + y0 - 1, 8 + step, 22 + y0 - 1), fill=LIGHT)
        d.rectangle((7 - step, 18 + y0 - 1, 9 - step, 22 + y0 - 1), fill=MID)
        d.rectangle((6 + step, 21 + y0 - 1, 8 + step, 22 + y0 - 1), fill=DARK)
    # body
    d.rectangle((3, 11 + y0, 12, 18 + y0), fill=WHITE)
    d.rectangle((3, 16 + y0, 12, 18 + y0), fill=LIGHT)
    if direction == 3:
        d.rectangle((4, 10 + y0, 11, 17 + y0), fill=LIGHT)
        d.rectangle((6, 12 + y0, 9, 14 + y0), fill=MID)
    if direction == 0:
        d.rectangle((6, 13 + y0, 9, 15 + y0), fill=MID)
        d.point((6, 13 + y0), fill=RED)
        d.point((9, 13 + y0), fill=GREEN)
    # arms
    if direction in (0, 3):
        d.rectangle((1, 12 + y0 + step, 2, 16 + y0 + step), fill=WHITE)
        d.rectangle((13, 12 + y0 - step, 14, 16 + y0 - step), fill=WHITE)
    elif direction == 1:
        d.rectangle((6 - step, 12 + y0, 8 - step, 16 + y0), fill=LIGHT)
        d.rectangle((11, 11 + y0, 13, 16 + y0), fill=LIGHT)  # backpack edge
    else:
        d.rectangle((7 + step, 12 + y0, 9 + step, 16 + y0), fill=LIGHT)
        d.rectangle((2, 11 + y0, 4, 16 + y0), fill=LIGHT)
    # helmet
    d.ellipse((2, y0 - 1, 13, y0 + 10), fill=WHITE)
    d.arc((2, y0 - 1, 13, y0 + 10), 20, 160, fill=LIGHT)
    # visor (separate layer so it can be tinted)
    if direction == 0:
        box = (4, y0 + 2, 11, y0 + 7)
    elif direction == 1:
        box = (2, y0 + 2, 8, y0 + 7)
    elif direction == 2:
        box = (7, y0 + 2, 13, y0 + 7)
    else:
        box = None
    if box:
        d.ellipse(box, fill=OUT)  # placeholder so the outline wraps the visor too
        v.ellipse(box, fill=WHITE)
        hx = box[0] + 2
        v.point((hx, box[1] + 1), fill=(255, 255, 255, 255))
    body = outline(body)
    # punch the visor hole back to a dark base so the tinted layer reads well
    bp, vp = body.load(), visor.load()
    for y in range(24):
        for x in range(16):
            if vp[x, y][3]:
                bp[x, y] = (30, 40, 70, 255)
    # visor highlight in a lighter tone (still tintable)
    return body, visor


def astronaut_sheets():
    body = new(64, 96)
    visor = new(64, 96)
    for direction in range(4):
        for frame in range(4):
            b, v = astro_frame(direction, frame)
            body.paste(b, (frame * 16, direction * 24), b)
            visor.paste(v, (frame * 16, direction * 24), v)
    save(body, "astronaut.png")
    save(visor, "astronaut_visor.png")


def hats():
    sheet = new(80, 16)
    imgs = []
    # antenna
    a = new(16, 16)
    d = ImageDraw.Draw(a)
    d.line((10, 4, 10, 13), fill=DARK)
    d.ellipse((8, 1, 12, 5), fill=YELLOW)
    imgs.append(outline(a))
    # star
    s = new(16, 16)
    d = ImageDraw.Draw(s)
    pts = []
    for i in range(10):
        r = 6 if i % 2 == 0 else 3
        ang = -math.pi / 2 + i * math.pi / 5
        pts.append((8 + r * math.cos(ang), 8 + r * math.sin(ang)))
    d.polygon(pts, fill=YELLOW)
    imgs.append(outline(s))
    # cat ears
    c = new(16, 16)
    d = ImageDraw.Draw(c)
    d.polygon([(2, 14), (4, 6), (7, 13)], fill=(255, 160, 180, 255))
    d.polygon([(9, 13), (12, 6), (14, 14)], fill=(255, 160, 180, 255))
    imgs.append(outline(c))
    # crown
    k = new(16, 16)
    d = ImageDraw.Draw(k)
    d.polygon([(2, 13), (2, 6), (5, 10), (8, 5), (11, 10), (14, 6), (14, 13)], fill=YELLOW)
    d.point((8, 9), fill=RED)
    imgs.append(outline(k))
    # alien buddy
    al = new(16, 16)
    d = ImageDraw.Draw(al)
    d.ellipse((3, 4, 13, 14), fill=(110, 240, 110, 255))
    d.ellipse((5, 7, 7, 9), fill=OUT)
    d.ellipse((9, 7, 11, 9), fill=OUT)
    d.line((5, 4, 4, 1), fill=(110, 240, 110, 255))
    d.line((11, 4, 12, 1), fill=(110, 240, 110, 255))
    imgs.append(outline(al))
    for i, im in enumerate(imgs):
        sheet.paste(im, (i * 16, 0), im)
    save(sheet, "hats.png")


# ------------------------------------------------------------------ tiles

MAP_PALETTES = {
    "earth": [(214, 168, 104), (200, 152, 92), (226, 182, 118), (186, 138, 80)],
    "moon": [(150, 154, 166), (136, 140, 152), (164, 168, 180), (120, 124, 138)],
    "mars": [(190, 92, 52), (172, 80, 44), (206, 108, 64), (150, 66, 36)],
    "orbit": [(150, 158, 178), (132, 140, 160), (170, 176, 196), (112, 120, 142)],
    "europa": [(190, 225, 245), (170, 210, 238), (210, 238, 252), (150, 196, 228)],
}


def tiles():
    for name, pal in MAP_PALETTES.items():
        rnd = random.Random(name)
        sheet = new(64, 16)
        for variant in range(4):
            t = Image.new("RGBA", (16, 16), pal[0] + (255,))
            p = t.load()
            for y in range(16):
                for x in range(16):
                    r = rnd.random()
                    if r < 0.18:
                        p[x, y] = pal[1] + (255,)
                    elif r < 0.26:
                        p[x, y] = pal[2] + (255,)
            if name == "orbit":
                d = ImageDraw.Draw(t)
                d.rectangle((0, 0, 15, 15), outline=pal[3] + (255,))
                d.point((2, 2), fill=pal[2] + (255,))
                d.point((13, 13), fill=pal[2] + (255,))
                if variant == 1:
                    d.line((4, 8, 11, 8), fill=(255, 170, 60, 255))
            elif name == "europa":
                d = ImageDraw.Draw(t)
                if variant in (1, 3):
                    d.line((0, 5 + variant, 15, 9 - variant), fill=(120, 170, 220, 255))
            else:
                # pebbles
                for _ in range(variant + 1):
                    x, y = rnd.randrange(1, 14), rnd.randrange(1, 14)
                    p[x, y] = pal[3] + (255,)
                    p[x + 1, y] = pal[3] + (255,)
                    p[x, y - 1] = pal[2] + (255,)
            sheet.paste(t, (variant * 16, 0))
        save(sheet, f"tiles_{name}.png")


def decor():
    # small rock, big rock, crater (grayscale → tinted per map in game)
    small = new(16, 16)
    d = ImageDraw.Draw(small)
    d.ellipse((3, 7, 13, 14), fill=(150, 150, 150, 255))
    d.ellipse((5, 7, 10, 10), fill=(185, 185, 185, 255))
    save(outline(small, (60, 60, 60, 255)), "rock_small.png")
    big = new(32, 32)
    d = ImageDraw.Draw(big)
    d.polygon([(4, 26), (6, 12), (14, 5), (24, 8), (29, 20), (26, 28)], fill=(140, 140, 140, 255))
    d.polygon([(8, 14), (14, 8), (20, 10), (16, 16)], fill=(185, 185, 185, 255))
    d.polygon([(18, 22), (26, 18), (27, 26)], fill=(110, 110, 110, 255))
    save(outline(big, (60, 60, 60, 255)), "rock_big.png")
    crater = new(32, 32)
    d = ImageDraw.Draw(crater)
    d.ellipse((1, 6, 30, 28), fill=(170, 170, 170, 255))
    d.ellipse((5, 9, 27, 26), fill=(110, 110, 110, 255))
    d.ellipse((7, 13, 25, 26), fill=(135, 135, 135, 255))
    save(crater, "crater.png")


# ------------------------------------------------------------------ modules

def habitat():
    im = new(48, 48)
    d = ImageDraw.Draw(im)
    d.ellipse((3, 30, 44, 46), fill=(0, 0, 0, 70))
    d.ellipse((4, 8, 43, 44), fill=WHITE)
    d.chord((4, 8, 43, 44), 0, 180, fill=LIGHT)
    d.rectangle((5, 30, 42, 33), fill=ORANGE)
    for cx in (12, 24, 35):
        d.ellipse((cx - 3, 18, cx + 3, 23), fill=GLASS)
        d.point((cx - 1, 19), fill=WHITE)
    d.rectangle((19, 34, 28, 44), fill=DARK)
    d.rectangle((20, 35, 27, 44), fill=(60, 70, 100, 255))
    d.line((24, 8, 24, 2), fill=MID)
    d.ellipse((21, 0, 27, 4), fill=WHITE)
    save(outline(im), "habitat.png")


def solar():
    im = new(32, 32)
    d = ImageDraw.Draw(im)
    d.rectangle((14, 22, 17, 30), fill=MID)
    d.polygon([(2, 6), (29, 3), (29, 22), (2, 25)], fill=BLUE)
    for i in range(1, 4):
        x = 2 + i * 27 / 4
        d.line((x, 6 - i * 0.7, x, 25 - i * 0.7), fill=BLUE_L)
    d.line((2, 15, 29, 12), fill=BLUE_L)
    d.line((4, 7, 10, 6), fill=(220, 240, 255, 255))
    save(outline(im), "solar.png")
    # dusty variant
    dusty = im.copy()
    p = dusty.load()
    rnd = random.Random(3)
    for y in range(32):
        for x in range(32):
            c = p[x, y]
            if c[3] and c[:3] in (BLUE[:3], BLUE_L[:3]) and rnd.random() < 0.7:
                p[x, y] = (180, 120, 80, 255)
    save(outline(dusty), "solar_dusty.png")


def battery():
    im = new(16, 16)
    d = ImageDraw.Draw(im)
    d.rectangle((2, 3, 13, 14), fill=DARK)
    d.rectangle((6, 1, 9, 3), fill=MID)
    for i in range(3):
        d.rectangle((4, 11 - i * 3, 11, 12 - i * 3), fill=GREEN)
    save(outline(im), "battery.png")


def oxygen():
    im = new(32, 32)
    d = ImageDraw.Draw(im)
    d.ellipse((2, 24, 30, 31), fill=(0, 0, 0, 60))
    for x0 in (3, 17):
        d.rounded_rectangle((x0, 4, x0 + 11, 28), radius=5, fill=WHITE)
        d.rectangle((x0, 12, x0 + 11, 16), fill=BLUE)
        d.line((x0 + 2, 6, x0 + 2, 26), fill=(255, 255, 255, 255))
    d.rectangle((14, 18, 17, 21), fill=MID)
    save(outline(im), "oxygen.png")


def greenhouse(ripe=False):
    im = new(48, 32)
    d = ImageDraw.Draw(im)
    d.rectangle((2, 22, 45, 30), fill=WHITE)
    d.rectangle((2, 27, 45, 30), fill=LIGHT)
    d.chord((2, 4, 45, 40), 180, 360, fill=(170, 240, 200, 255))
    rnd = random.Random(7)
    for x in range(6, 43, 6):
        d.ellipse((x - 3, 12, x + 3, 22), fill=GREEN)
        d.ellipse((x - 2, 15, x + 1, 19), fill=GREEN_D)
        if ripe:
            d.point((x + rnd.randint(-1, 1), 14 + rnd.randint(0, 3)), fill=RED)
            d.point((x + rnd.randint(-2, 2), 17 + rnd.randint(0, 3)), fill=RED)
    d.arc((2, 4, 45, 40), 200, 260, fill=WHITE)
    save(outline(im), "greenhouse_ripe.png" if ripe else "greenhouse.png")


def shield():
    im = new(32, 32)
    d = ImageDraw.Draw(im)
    d.ellipse((4, 24, 28, 31), fill=(0, 0, 0, 60))
    d.polygon([(8, 28), (12, 12), (20, 12), (24, 28)], fill=LIGHT)
    d.rectangle((13, 12, 18, 27), fill=WHITE)
    d.ellipse((9, 2, 22, 14), fill=CYAN)
    d.ellipse((12, 4, 16, 8), fill=(220, 255, 250, 255))
    save(outline(im), "shield.png")


def lab():
    im = new(32, 32)
    d = ImageDraw.Draw(im)
    d.rectangle((2, 8, 29, 29), fill=WHITE)
    d.polygon([(1, 9), (16, 2), (30, 9)], fill=LIGHT)
    d.rectangle((12, 19, 19, 29), fill=ORANGE)
    d.rectangle((4, 12, 10, 17), fill=GLASS)
    d.rectangle((21, 12, 27, 17), fill=GLASS)
    # flask sign
    d.polygon([(14, 10), (17, 10), (19, 16), (12, 16)], fill=(160, 120, 255, 255))
    save(outline(im), "lab.png")


def antenna():
    im = new(16, 32)
    d = ImageDraw.Draw(im)
    d.line((8, 12, 8, 30), fill=MID)
    d.line((5, 30, 11, 30), fill=DARK)
    d.pieslice((1, 1, 15, 17), 200, 340, fill=WHITE)
    d.line((8, 8, 8, 3), fill=DARK)
    d.point((8, 2), fill=RED)
    save(outline(im), "antenna.png")


def overlays():
    sp = new(16, 16)
    d = ImageDraw.Draw(sp)
    d.polygon([(9, 1), (4, 9), (8, 9), (6, 15), (12, 6), (8, 6)], fill=YELLOW)
    save(outline(sp, (180, 40, 30, 255)), "spark.png")
    lk = new(16, 16)
    d = ImageDraw.Draw(lk)
    for i, r in enumerate((6, 4, 2)):
        d.ellipse((8 - r, 8 - r, 8 + r, 8 + r), outline=(255, 255, 255, 200 - i * 50))
    save(lk, "leak.png")
    ghost = new(16, 16)
    d = ImageDraw.Draw(ghost)
    d.rectangle((0, 0, 15, 15), fill=(255, 255, 255, 90), outline=(255, 255, 255, 255))
    save(ghost, "ghost.png")
    # shield bubble ring
    size = 96
    bub = new(size, size)
    p = bub.load()
    c = (size - 1) / 2
    for y in range(size):
        for x in range(size):
            r = math.hypot(x - c, y - c) / c
            if r <= 1.0:
                a = int(30 + 150 * max(0.0, (r - 0.8) / 0.2) ** 2) if r > 0.8 else 28
                p[x, y] = (110, 255, 230, a)
    save(bub, "bubble.png")
    dot = new(4, 4)
    ImageDraw.Draw(dot).ellipse((0, 0, 3, 3), fill=(255, 255, 255, 255))
    save(dot, "dot.png")
    arrow = new(16, 16)
    ImageDraw.Draw(arrow).polygon([(8, 1), (15, 9), (11, 9), (11, 15), (5, 15), (5, 9), (1, 9)], fill=YELLOW)
    save(outline(arrow), "arrow.png")


# ------------------------------------------------------------------ sites & vehicles

def sites():
    for f in range(2):
        im = new(16, 16)
        d = ImageDraw.Draw(im)
        d.ellipse((3, 12, 13, 15), fill=(0, 0, 0, 60))
        d.polygon([(8, 1), (12, 7), (8, 13), (4, 7)], fill=(110, 230, 255, 255))
        d.polygon([(8, 1), (12, 7), (8, 7)], fill=(190, 250, 255, 255))
        if f == 1:
            d.point((13, 2), fill=(255, 255, 255, 255))
            d.point((2, 10), fill=(255, 255, 255, 255))
        save(outline(im), f"sample_{f}.png")
    ice = new(16, 16)
    d = ImageDraw.Draw(ice)
    d.polygon([(2, 13), (4, 5), (9, 3), (14, 7), (13, 14)], fill=(200, 240, 255, 255))
    d.polygon([(5, 6), (9, 4), (10, 8)], fill=(255, 255, 255, 255))
    save(outline(ice), "ice.png")
    wreck = new(16, 16)
    d = ImageDraw.Draw(wreck)
    d.polygon([(1, 12), (6, 6), (9, 10)], fill=MID)
    d.rectangle((8, 8, 14, 13), fill=LIGHT)
    d.line((9, 9, 13, 12), fill=ORANGE)
    d.ellipse((3, 3, 6, 6), fill=DARK)
    save(outline(wreck), "wreck.png")


def rover():
    im = new(32, 24)
    d = ImageDraw.Draw(im)
    for x in (4, 13, 22):
        d.ellipse((x, 15, x + 6, 22), fill=DARK)
        d.point((x + 3, 18), fill=MID)
    d.rectangle((2, 8, 29, 16), fill=(230, 110, 60, 255))
    d.rectangle((17, 3, 27, 9), fill=GLASS)
    d.line((6, 8, 6, 2), fill=MID)
    d.ellipse((4, 0, 9, 3), fill=WHITE)
    save(outline(im), "rover.png")


def cargo():
    im = new(32, 40)
    d = ImageDraw.Draw(im)
    d.polygon([(16, 1), (26, 14), (6, 14)], fill=LIGHT)
    d.rectangle((6, 14, 25, 33), fill=WHITE)
    d.rectangle((6, 20, 25, 23), fill=ORANGE)
    d.polygon([(6, 30), (1, 38), (8, 34)], fill=MID)
    d.polygon([(25, 30), (30, 38), (23, 34)], fill=MID)
    d.rectangle((11, 25, 20, 31), fill=DARK)
    save(outline(im), "cargo.png")
    fl = new(16, 16)
    d = ImageDraw.Draw(fl)
    d.polygon([(3, 0), (13, 0), (8, 15)], fill=(255, 150, 40, 255))
    d.polygon([(5, 0), (11, 0), (8, 9)], fill=(255, 240, 150, 255))
    save(fl, "flame.png")


def app_icon():
    im = Image.new("RGBA", (64, 64), (14, 18, 48, 255))
    d = ImageDraw.Draw(im)
    d.ellipse((34, 6, 58, 30), fill=(200, 90, 50, 255))
    d.ellipse((40, 10, 46, 15), fill=(170, 70, 40, 255))
    for (x, y) in ((8, 8), (20, 4), (28, 20), (6, 30), (56, 40)):
        d.point((x, y), fill=(255, 255, 255, 255))
    b, v = astro_frame(0, 0)
    vt = Image.new("RGBA", v.size, (0, 0, 0, 0))
    vp, vtp = v.load(), vt.load()
    for y in range(24):
        for x in range(16):
            if vp[x, y][3]:
                vtp[x, y] = (255, 200, 70, 255)
    big = b.resize((32, 48), Image.NEAREST)
    bigv = vt.resize((32, 48), Image.NEAREST)
    im.paste(big, (6, 14), big)
    im.paste(bigv, (6, 14), bigv)
    im.resize((256, 256), Image.NEAREST).save(os.path.join(OUT_DIR, "app_icon.png"))


def rtg():
    im = new(32, 32)
    d = ImageDraw.Draw(im)
    d.ellipse((4, 24, 28, 31), fill=(0, 0, 0, 60))
    for i in range(5):
        x = 6 + i * 5
        d.rectangle((x, 6, x + 2, 26), fill=DARK)
    d.rounded_rectangle((10, 3, 22, 28), radius=4, fill=LIGHT)
    d.ellipse((12, 10, 20, 18), fill=YELLOW)
    d.polygon([(16, 14), (13, 11), (19, 11)], fill=OUT)
    d.polygon([(16, 14), (12, 16), (14, 18)], fill=OUT)
    d.polygon([(16, 14), (20, 16), (18, 18)], fill=OUT)
    save(outline(im), "rtg.png")


def ore():
    im = new(16, 16)
    d = ImageDraw.Draw(im)
    d.ellipse((2, 12, 14, 15), fill=(0, 0, 0, 60))
    d.polygon([(2, 13), (3, 7), (7, 4), (11, 6), (14, 12)], fill=(120, 110, 105, 255))
    d.polygon([(9, 13), (11, 9), (14, 12)], fill=(95, 88, 84, 255))
    for (x, y) in ((5, 8), (8, 6), (10, 10), (6, 11)):
        d.point((x, y), fill=(255, 180, 70, 255))
        d.point((x + 1, y), fill=(255, 220, 120, 255))
    save(outline(im), "ore.png")


if __name__ == "__main__":
    astronaut_sheets()
    hats()
    tiles()
    decor()
    habitat()
    solar()
    battery()
    oxygen()
    greenhouse(False)
    greenhouse(True)
    shield()
    lab()
    antenna()
    overlays()
    sites()
    rover()
    cargo()
    rtg()
    ore()
    app_icon()
    print("sprites:", sorted(os.listdir(OUT_DIR)))
