# Vulture — a mod for Minecraft Horizon

A vulture for [Minecraft Horizon](https://playhorizon.dev), the Rust reimplementation of Minecraft Java 26.3: spawn it,
ride it and fly. Planned: taming with thrown raw chicken, flight stamina, a harness, two chests, talons, breeding, stats
and variants (see [docs/SPECS.md](docs/SPECS.md)).

The game text is in English and French.

> **Status:** prototype. What works and what is left: [docs/SPECS.md, §0](docs/SPECS.md#0-progress-as-of-2026-11-07).
> Single player only for now; multiplayer comes later.

## What works

- The tawny vulture, `vulture:vulture`: `/summon vulture:vulture` or the **spawn egg** ("Vulture" creative tab).
- **Mount**: right-click it (no cheats needed). **Get off**: hold sneak (to confirm after mounting by right-click), or
  `/ride @s dismount`.
- **Fly**:

  | Key | Effect |
  |---|---|
  | W A S D | Direction following your look |
  | Space | Climb, flapping |
  | (nothing) | Glide, slowly losing height |
  | Left Ctrl | Dive |

- Its own model and texture, animated (wings folded or flapping, head, legs), and its own sounds (hisses, hurt, death,
  wing flaps).

## Install and test

You need the [Horizon launcher](https://playhorizon.dev) with a test instance (here `vulture-dev`), Rust (with
`cargo horizon-mod`, from Horizon's sources); ffmpeg to remake the sounds.

```sh
cargo install --locked --git https://github.com/horizon-rust/horizon cargo-horizon-mod
cargo vulture install           # or: cargo vulture install <instance>
```

It builds both halves of the mod, installs them in the instance and runs its native build (a minute or two,
longer the first time):

- `content/` → `vulture.hmod`, the content part: entity, egg, textures, sounds, translations;
- `native/` → `vulture.hnmod`, the native Rust part: flight, rendering, sounds, right-click mounting, compiled into the
  game by the launcher.

Then, in the launcher, pick the instance and press **Play** (restart the launcher if it was open during the build, so
it sees the new version). The native mod is pinned to one Horizon revision (`native/horizon-mixin.toml`), because it
reads internal game fields.

## Repository

| Path | Contents |
|---|---|
| `content/` | The content mod (`horizon-mod.toml`, `data/`, `assets/`) |
| `native/` | The native mod: `api/` (the mod's Rust crate) and two mixin injections (`mount_click.rs`, `mount_hud.rs`) |
| `assets/models/vulture.bbmodel` | The Blockbench model, the source of the in-game model |
| `tools/` | The build tools, in Rust, run as `cargo vulture <command>` (alias in `.cargo/config.toml`) |
| `docs/SPECS.md` | The specification and progress |

The tools:

| Command | What it does |
|---|---|
| `cargo vulture model` | Converts `assets/models/vulture.bbmodel` into `native/api/src/model.rs` (after editing the model in Blockbench) |
| `cargo vulture sounds [folder]` | Makes the `.ogg` files from the freesound downloads (default folder `sounds-src/`) |
| `cargo vulture pack` | Packages `native/` into `target/vulture.hnmod` |
| `cargo vulture install [instance]` | Builds and packages both halves, installs them in the instance and runs its native build |

## Sounds

Made from [CC0](https://creativecommons.org/publicdomain/zero/1.0/) sounds from [freesound.org](https://freesound.org)
(hissing geese, wings, a vulture imitation), edited; the list is in `tools/src/sounds.rs`.
