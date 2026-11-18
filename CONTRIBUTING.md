# Contributing to Sunrise Creatures

How the repository is organised, how to build and test the mods, and the rules a change follows. Players: see the
[README](README.md).

## Documentation

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): the modules, their dependencies and responsibilities, what goes in the
  library (the rule of two), the creative tab, building, adding a mod.
- [docs/CONVENTIONS.md](docs/CONVENTIONS.md): ids, translations, specs, code and assets conventions.
- [docs/SUNRISE-ISSUES.md](docs/SUNRISE-ISSUES.md): the Sunrise issues we raised or follow, their status, and the
  workaround each one left in our code. Keep it up to date.
- One `SPECS.md` per module, the source of truth for it, its §0 the current status (the library's:
  [common/SPECS.md](common/SPECS.md)).

## Repository

| Path | Contents |
|---|---|
| `common/` | The library, `creature`: its package at the root (`sunrise-mod.toml`; `Cargo.toml` and `src/`, the crate `mod-creature`; `assets/creature/`, `data/creature/`: the jar, the guide, the creative tab, translations), `module.toml`, `SPECS.md` |
| `mods/<mod>/` | One mod: its package at the root (`sunrise-mod.toml`; `Cargo.toml` and `src/`, the crate `mod-<mod>`; `assets/<mod>/`, `data/<mod>/`: entities, items, recipes, loot, spawns, textures, sounds, translations), `sources/` (`models/*.bbmodel`, `sounds.toml`, Horizon's old manifests while it is ported), `module.toml`, `SPECS.md`, `README.md` (its page for players) |
| `tools/` | The build tools, `cargo creature <command>` |
| `docs/` | Series-wide documentation |
| `.github/workflows/ci.yml` | CI (below) |
| `.github/dependabot.yml` | Dependabot (below) |

Generated files are committed and checked by `cargo creature check`: the creative tab
(`common/data/creature/sunrise/creative_tab/creature.json`), each model's Rust (`src/model*.rs`) and
the models' pictures (`docs/models/`). Never edit them by hand.

## Build and test

You need the [Sunrise launcher](https://playsunrise.net) (signed in, with a test instance, here `creature-dev`; any
name works with `--instance`), Rust, `cargo sunrise-mod` and ffmpeg to remake the sounds.

```sh
cargo install --locked --git https://github.com/sunrise-org/sunrise cargo-sunrise-mod
cargo creature install                                   # the library and every mod, in creature-dev
cargo creature install vulture --instance my-instance    # one mod (with the library and what it requires)
```

The tools find the launcher by `$SUNRISE_LAUNCHER`, else `sunrise-launcher` on the `PATH`, else a
`sunrise-launcher*.AppImage` in your download folder, `~/Downloads`, `~/Applications`, `~/.local/bin` or `~/bin`; its
data by `$SUNRISE_LAUNCHER_DATA`, else `$XDG_DATA_HOME/sunrise` or `~/.local/share/sunrise`.

`install` writes the creative tab and packages each module (staged in `target/stage/`, zipped to `target/<id>.smod`).
A module whose Rust is on Sunrise's API (`ported = true` in its `module.toml`) is built by the launcher
(`sunrise-launcher native-build`, a few seconds once its tools are downloaded), which also makes that build the one the
instance plays; a content-only module goes in the instance's `sunrise-mods/`. Then press **Play** on the instance.

The modules are **pinned** to one Sunrise build, the same for every module (`[native] sunrise` and `mixins` in
`sunrise-mod.toml`, CI's `SUNRISE_REV`); the launcher fetches that build's SDK and compiles against it.

| Command | What it does |
|---|---|
| `cargo creature list` | The library and the mods, with their status |
| `cargo creature tab` | Writes the series' creative tab from the modules' `module.toml`, and the library's recipe table (`recipes.rs`, for the guide's Items tab) from their recipe files |
| `cargo creature model <mod> [model]` | Converts a Blockbench model (`sources/models/`) into Rust (`src/model.rs`, or `model_<model>.rs`) |
| `cargo creature preview` | Redraws every model's pictures in `docs/models/` (shown by the READMEs and specs) |
| `cargo creature sounds <mod> [folder]` | Cuts a mod's `.ogg` files from freesound downloads (default `sounds-src/`, any sample rate), as its `sources/sounds.toml` says |
| `cargo creature vanilla [jar]` | Writes the Vanilla Bestiary's `game_data.rs` from Minecraft's jar (default: the launcher's 26.3): natural spawns, drops, breeding foods |
| `cargo creature pack <module>` | Stages a module's package in `target/stage/` and zips it into `target/<id>.smod` |
| `cargo creature build [mod...]` | Packages and checks (`cargo sunrise-mod check`) the library and the mods into `target/dist/`, without installing |
| `cargo creature check` | The repository's consistency: tab, manifests, pins, translations, JSON, generated files, sounds (events, files, subtitles), recipe book advancements, docs |
| `cargo creature native-check [sdk]` | Clippy and the unit tests on every ported crate against the pinned Sunrise SDK (fetched into `target/sunrise-sdk/`) |
| `cargo creature install [--instance <name>] [mod...]` | Builds and installs, then runs the instance's native build |

## Before committing

- `rustfmt` on the tools and the modules' `src/` (`rustfmt.toml`), Clippy with `-D warnings` and the tests on the tools
  (`cargo test`) and the native crates (`cargo creature native-check`), `cargo creature check`.
- A new recipe comes with its recipe book advancement (`advancement/recipes/<tab>/<name>.json`, unlocked by one of its
  ingredients and rewarding it), or the recipe book never shows it; `cargo creature check` says which lack one. A
  feather in a recipe is `#creature:feathers`.
- A new sound: its source in the mod's `sources/sounds.toml`, `cargo creature sounds <mod>`, its event in `sounds.json`
  with a subtitle in both languages.
- Pure logic (aiming, conversions, text built for the game) gets a unit test beside it (`#[cfg(test)]`).
- After a native change: `cargo creature install` and a test in game, along the module's test list (its `SPECS.md`,
  "Testing in game").
- After a model change: `cargo creature model <mod> [model]` and `cargo creature preview`.
- Everything in the repository is in English. The game text is translated: keep each module's
  `assets/<id>/lang/en_us.json` and `fr_fr.json` in step. No em dash anywhere.
- A change to a module updates its `SPECS.md` §0 (the current status, not a log) and, when players see it, its
  `README.md`.

CI (`.github/workflows/ci.yml`) runs all of this on every push and pull request, then builds every module and keeps the
files as an artifact.

Dependabot (`.github/dependabot.yml`) opens weekly pull requests for the CI's actions (all in one) and the tools' Rust
dependencies (minor and patch updates in one, each major one apart); CI checks them like any other. The native crates
are left out: Horizon, their only dependency, is pinned for the whole series and moved by hand.

## Rules of the series

- **The rule of two**: a system used by two or more mods goes in the library; species content stays in its mod. Never
  duplicate a system between mods.
- **No mod requires another**: optional content between mods goes through conditional recipes
  (`neoforge:conditions`, `mod_loaded`, `not`). A library feature some mods need (the glass jar) switches on with them.
- **One creative tab**, the library's, generated from each module's `module.toml` (`creative` list). Mods declare none.
- **Sunrise issues**: a missing API or a bug becomes an issue on
  [sunrise-org/sunrise](https://github.com/sunrise-org/sunrise/issues) and a workaround marked in the code with its
  issue, listed in `docs/SUNRISE-ISSUES.md`.

Adding a mod: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), "Adding a mod".
