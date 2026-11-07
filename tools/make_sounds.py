#!/usr/bin/env python3
"""Fabrique les sons du vautour à partir des fichiers freesound (CREDITS.md).

    python3 tools/make_sounds.py [dossier des téléchargements, par défaut sounds-src/]

Les fichiers téléchargés gardent le nom que freesound leur donne (`740195__el_boss__goose-hiss.wav`) :
on les retrouve par leur numéro. Sortie : `content/assets/vulture/sounds/vulture/*.ogg`, mono 44,1 kHz,
volume normalisé, comme les sons vanilla. Il faut ffmpeg.
"""
import glob
import json
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "sounds-src")
OUT = os.path.join(ROOT, "content", "assets", "vulture", "sounds", "vulture")

GOOSE_A = 740195   # el_boss, Goose Hiss
GOOSE_B = 844558   # TheKingOfGeeks360, White Chinese Goose; Hiss
CALL = 188041      # AntumDeluge, Vulture (imitation)
WINGS = 670509     # 2create, Huge wing flaps
TAKEOFF = 389634   # _stubb, Wing Flap 1
EAT = 234237       # ProductionNow, Eating Juicy Meat


def source(number):
    found = glob.glob(os.path.join(SRC, f"{number}__*")) + glob.glob(os.path.join(SRC, f"{number}.*"))
    if not found:
        sys.exit(f"il manque le son freesound {number} dans {SRC}")
    return found[0]


def regions(path, noise="-35dB", gap=0.08):
    """Les passages sonores d'un fichier : [(début, fin)] en secondes (`silencedetect`)."""
    log = subprocess.run(
        ["ffmpeg", "-hide_banner", "-i", path, "-af", f"silencedetect=n={noise}:d={gap}", "-f", "null", "-"],
        capture_output=True, text=True).stderr
    h, m, sec = re.search(r"Duration: (\d+):(\d+):([\d.]+)", log).groups()
    length = int(h) * 3600 + int(m) * 60 + float(sec)
    starts = [float(x) for x in re.findall(r"silence_start: ([\d.]+)", log)]
    ends = [float(x) for x in re.findall(r"silence_end: ([\d.]+)", log)]
    out, at = [], 0.0
    for s, e in zip(starts, ends + [length]):
        if s - at > 0.05:
            out.append((at, s))
        at = e
    if length - at > 0.05:
        out.append((at, length))
    return out


def make(name, path, start=0.0, length=None, pitch=1.0, fade=0.15, gain_db=0.0):
    """Un extrait, transposé de `pitch` (plus lent et plus grave en dessous de 1)."""
    filters = [f"atrim=start={start}" + (f":duration={length * pitch}" if length else ""),
               "asetpts=PTS-STARTPTS",
               f"asetrate=44100*{pitch},aresample=44100",
               "highpass=f=60", "loudnorm=I=-16:TP=-1.5"]
    if gain_db:
        filters.append(f"volume={gain_db}dB")
    if length:
        filters.append(f"afade=t=out:st={max(length - fade, 0)}:d={fade}")
    out = os.path.join(OUT, f"{name}.ogg")
    subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-i", path,
                    "-af", ",".join(filters), "-ac", "1", "-ar", "44100",
                    "-c:a", "libvorbis", "-q:a", "5", out], check=True)
    print(f"{name}.ogg  ← {os.path.basename(path)} [{start:.2f}s, ×{pitch}]")


def longest(path, n):
    """Les `n` passages sonores les plus longs, dans l'ordre du fichier."""
    best = sorted(regions(path), key=lambda r: r[1] - r[0], reverse=True)[:n]
    return sorted(best)


def main():
    os.makedirs(OUT, exist_ok=True)
    a, b = source(GOOSE_A), source(GOOSE_B)
    # Le vautour souffle plus grave qu'une oie : environ 4 demi-tons plus bas (×0,79).
    hiss_a, hiss_b = longest(a, 2), longest(b, 2)
    hisses = [(a, r) for r in hiss_a] + [(b, r) for r in hiss_b]
    for i, (path, (s, e)) in enumerate(hisses[:3], 1):
        make(f"ambient{i}", path, s, min(e - s, 2.0), pitch=0.79)
    make("call", source(CALL), pitch=0.9)
    # Blessé : un souffle bref, moins transposé ; mort : le plus long, plus grave, qui s'éteint.
    for i, (path, (s, e)) in enumerate(hisses[:2], 1):
        make(f"hurt{i}", path, s, min(e - s, 0.5), pitch=0.9, fade=0.1)
    path, (s, e) = max(hisses, key=lambda h: h[1][1] - h[1][0])
    make("death", path, s, min(e - s, 2.5), pitch=0.65, fade=0.8)
    # Battements : les trois coups les plus francs des grandes ailes, un coup par fichier.
    # (S'il y en a moins de trois, on reprend les premiers un peu plus graves.)
    flaps = longest(source(WINGS), 3)
    for i in range(3):
        s, e = flaps[i % len(flaps)]
        make(f"flap{i + 1}", source(WINGS), s, min(e - s, 0.7), pitch=1.0 - 0.08 * (i // len(flaps)),
             fade=0.1, gain_db=-3)
    make("takeoff", source(TAKEOFF), 0.0, 1.5, fade=0.4)
    for i, (s, e) in enumerate(longest(source(EAT), 2), 1):
        make(f"eat{i}", source(EAT), s, min(e - s, 0.8), fade=0.1, gain_db=-4)
    # Tous les fichiers que sounds.json nomme doivent exister.
    with open(os.path.join(ROOT, "content", "assets", "vulture", "sounds.json")) as f:
        events = json.load(f)
    for event in events.values():
        for entry in event["sounds"]:
            name = (entry["name"] if isinstance(entry, dict) else entry).split("/")[-1]
            if not os.path.exists(os.path.join(OUT, f"{name}.ogg")):
                sys.exit(f"sounds.json nomme {name}.ogg, qui n'a pas été fait")


if __name__ == "__main__":
    main()
