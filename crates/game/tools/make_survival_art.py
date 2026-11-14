#!/usr/bin/env python3
"""Art for the survival part (landing + Moon base): the coin icon and the lunar lander sprite.

Original art made for this project — released under CC0 1.0.
Run:  python3 crates/game/tools/make_survival_art.py
"""
import os
from PIL import Image, ImageDraw

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
OUT = (24, 20, 37, 255)


def outline(img, color=OUT):
    w, h = img.size
    src = img.load()
    res = img.copy()
    dst = res.load()
    for y in range(h):
        for x in range(w):
            if src[x, y][3] == 0:
                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    nx, ny = x + dx, y + dy
                    if 0 <= nx < w and 0 <= ny < h and src[nx, ny][3] > 0:
                        dst[x, y] = color
                        break
    return res


def coin():
    s = 18
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.ellipse((1, 1, s - 2, s - 2), fill=(214, 150, 40, 255))
    d.ellipse((2, 2, s - 3, s - 4), fill=(250, 196, 64, 255))
    d.ellipse((5, 5, s - 6, s - 6), outline=(214, 150, 40, 255))
    # a small star in the middle
    c = s // 2
    for x, y in ((c, c - 2), (c - 1, c - 1), (c, c - 1), (c + 1, c - 1), (c - 2, c), (c - 1, c), (c, c), (c + 1, c), (c + 2, c), (c - 1, c + 1), (c + 1, c + 1)):
        img.putpixel((x, y), (255, 240, 170, 255))
    img.putpixel((4, 4), (255, 250, 220, 255))
    img = outline(img)
    img.resize((72, 72), Image.NEAREST).save(os.path.join(ROOT, "assets", "icons", "coin.png"))


def lander():
    w, h = 24, 24
    img = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    gold = (226, 170, 52, 255)
    gold2 = (178, 120, 34, 255)
    grey = (196, 200, 214, 255)
    dark = (96, 100, 120, 255)
    # ascent stage (cabin)
    d.rectangle((8, 3, 15, 9), fill=grey)
    d.rectangle((9, 4, 11, 6), fill=(70, 120, 200, 255))
    d.rectangle((13, 2, 14, 3), fill=dark)
    # descent stage wrapped in gold foil
    d.rectangle((6, 10, 17, 15), fill=gold)
    d.line((6, 15, 17, 15), fill=gold2)
    d.line((6, 12, 17, 12), fill=gold2)
    # engine bell
    d.rectangle((10, 16, 13, 17), fill=dark)
    # legs and pads
    d.line((6, 13, 2, 20), fill=grey)
    d.line((17, 13, 21, 20), fill=grey)
    d.line((1, 21, 3, 21), fill=grey)
    d.line((20, 21, 22, 21), fill=grey)
    img = outline(img)
    img.save(os.path.join(ROOT, "assets", "sprites", "lander.png"))


if __name__ == "__main__":
    coin()
    lander()
    print("coin.png, lander.png written")
