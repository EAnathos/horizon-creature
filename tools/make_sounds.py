#!/usr/bin/env python3
"""Fabrique les sons du vautour à partir de fichiers freesound, tous en CC0 (domaine public : rien à créditer).

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


def main():
    os.makedirs(OUT, exist_ok=True)
    a, b = source(GOOSE_A), source(GOOSE_B)
    # Découpes choisies sur les passages que `regions` trouve (seuils de -22 à -35 dB) : le souffle
    # long de chaque oie, puis les bouffées courtes qui suivent celui d'el_boss (1,57 s et 2,79 s).
    # Le vautour souffle plus grave qu'une oie : environ 4 demi-tons plus bas (×0,79).
    make("ambient1", a, 0.0, 1.7, pitch=0.79)
    make("ambient2", b, 0.0, 1.95, pitch=0.79)
    make("ambient3", a, 1.5, 2.0, pitch=0.79, fade=0.3)
    make("call", source(CALL), pitch=0.9)
    # Blessé : une bouffée brève, moins transposée ; mort : le souffle le plus long, plus grave, qui s'éteint.
    make("hurt1", a, 1.55, 0.35, pitch=0.9, fade=0.08)
    make("hurt2", a, 2.77, 0.35, pitch=0.9, fade=0.08)
    make("death", b, 0.0, 2.4, pitch=0.65, fade=0.9)
    # Battements : trois coups francs des grandes ailes, avec leur souffle.
    wings = source(WINGS)
    for i, (start, length) in enumerate([(1.0, 0.5), (2.42, 0.7), (3.7, 0.5)], 1):
        make(f"flap{i}", wings, start, length, fade=0.12, gain_db=-3)
    make("takeoff", source(TAKEOFF), 0.0, 0.93, fade=0.3)
    eat = source(EAT)
    make("eat1", eat, 0.22, 0.95, fade=0.1, gain_db=-4)
    make("eat2", eat, 5.4, 0.7, fade=0.1, gain_db=-4)
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
