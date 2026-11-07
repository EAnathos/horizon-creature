# Vautour — mod pour Minecraft Horizon

Un vautour pour [Minecraft Horizon](https://playhorizon.dev), la réimplémentation en Rust de Minecraft Java 26.3 :
on le fait apparaître, on monte dessus et on vole. À terme : apprivoisement au poulet cru lancé, endurance en vol,
harnais, deux coffres, serres, reproduction, statistiques et variantes (voir [docs/SPECS.md](docs/SPECS.md)).

> **État :** prototype. Ce qui marche et ce qui reste à faire : [docs/SPECS.md, §0](docs/SPECS.md#0-avancement-au-2026-11-07).
> Testé en solo seulement.

## Ce qui marche

- Le vautour fauve, `vulture:vulture` : `/summon vulture:vulture` ou l'**œuf d'apparition** (onglet créatif « Vautour »).
- **Monter** : clic droit dessus (sans cheats). **Descendre** : rester accroupi (à confirmer après une montée par clic droit), ou `/ride @s dismount`.
- **Voler** :

  | Touche | Effet |
  |---|---|
  | Z Q S D | Direction selon le regard |
  | Espace | Monter en battant des ailes |
  | (rien) | Plané, en descendant lentement |
  | Ctrl gauche | Piqué |

- Son modèle et sa texture, animés (ailes repliées ou battantes, tête, pattes), et ses sons (sifflements, blessure, mort,
  battements d'ailes).

## Installer et tester

Il faut le [launcher Horizon](https://playhorizon.dev) avec une instance de test (ici `vulture-dev`), Rust (l'outil
`cargo horizon-mod` des sources d'Horizon) et Python 3 ; ffmpeg pour refaire les sons.

```sh
cargo install --locked --git https://github.com/horizon-rust/horizon cargo-horizon-mod
tools/install-dev.sh            # ou : tools/install-dev.sh <instance>
```

Le script construit les deux moitiés du mod et les installe dans l'instance, puis lance sa build native (une à deux
minutes, plus la première fois) :

- `content/` → `vulture.hmod`, la partie contenu : l'entité, l'œuf, la texture, les sons, les traductions ;
- `native/` → `vulture.hnmod`, la partie native en Rust : vol, rendu, sons, clic droit, compilée dans le jeu par le
  launcher.

Ensuite, dans le launcher : choisir l'instance et **Jouer** (s'il était ouvert pendant la build, le relancer pour qu'il
voie la nouvelle version). Le mod natif est épinglé sur une révision d'Horizon (`native/horizon-mixin.toml`), car il
lit des champs internes du jeu.

## Le dépôt

| Chemin | Contenu |
|---|---|
| `content/` | Le mod de contenu (`horizon-mod.toml`, `data/`, `assets/`) |
| `native/` | Le mod natif : `api/` (le crate Rust du mod), deux injections mixin (`mount_click.rs`, `mount_hud.rs`) |
| `assets/models/vulture.bbmodel` | Le modèle Blockbench, la source du modèle du jeu |
| `tools/bbmodel_to_rust.py` | Convertit le modèle en `native/api/src/model.rs` |
| `tools/make_sounds.py` | Fabrique les `.ogg` à partir des fichiers freesound (à mettre dans `sounds-src/`) |
| `tools/pack_native.py`, `tools/install-dev.sh` | Empaquettent et installent le mod |
| `docs/SPECS.md` | Les spécifications et l'avancement |

Après une modification du modèle dans Blockbench : `python3 tools/bbmodel_to_rust.py`, puis `tools/install-dev.sh`.

## Sons

Tirés de sons [CC0](https://creativecommons.org/publicdomain/zero/1.0/) de [freesound.org](https://freesound.org)
(oies qui sifflent, ailes, imitation de vautour), retouchés ; la liste est dans `tools/make_sounds.py`.
