#!/usr/bin/env python3
"""Generates a calm chiptune loop (assets/sounds/music.wav). Original, CC0 1.0."""
import math, os, struct, wave
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
SR, BPM = 22050, 96
beat = 60 / BPM
def note(n):  # MIDI number -> Hz
    return 440 * 2 ** ((n - 69) / 12)
chords = [(57, 60, 64), (53, 57, 60), (48, 52, 55), (55, 59, 62)]  # Am F C G
melody = [76, 74, 72, 74, 76, 79, 76, 74, 72, 69, 72, 74, 72, 71, 67, 69]
total = int(SR * beat * 4 * len(chords) * 2)
buf = [0.0] * total
def add(freq, start, dur, vol, kind):
    s0, n = int(start * SR), int(dur * SR)
    for i in range(n):
        if s0 + i >= total: break
        t = i / SR
        ph = (freq * t) % 1
        v = (1 if ph < 0.5 else -1) * 0.5 if kind == "sq" else (4 * abs(ph - 0.5) - 1)
        env = min(1, i / 300) * (1 - i / n) ** 1.5
        buf[s0 + i] += v * vol * env
for rep in range(2):
    for ci, ch in enumerate(chords):
        bar = (rep * 4 + ci) * 4 * beat
        add(note(ch[0] - 12), bar, beat * 4, 0.22, "tri")
        for k in range(8):
            add(note(ch[k % 3] + 12), bar + k * beat / 2, beat / 2, 0.06, "sq")
        for k in range(4):
            m = melody[(ci * 4 + k) % len(melody)]
            if rep == 1 or k % 2 == 0:
                add(note(m), bar + k * beat, beat * 0.9, 0.09, "tri")
peak = max(abs(x) for x in buf) or 1
with wave.open(os.path.join(ROOT, "assets", "sounds", "music.wav"), "w") as w:
    w.setnchannels(1); w.setsampwidth(2); w.setframerate(SR)
    w.writeframes(b"".join(struct.pack("<h", int(x / peak * 0.8 * 32000)) for x in buf))
print("music.wav", total / SR, "s")
