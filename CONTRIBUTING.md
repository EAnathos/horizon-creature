# Contributing to Horizon Creature

How the repository is organised, how to build and test the mods, and the rules a change follows. Players: see the
[README](README.md).

## Documentation

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): the modules, their dependencies and responsibilities, what goes in the
  library (the rule of two), the creative tab, building, adding a mod.
- [docs/CONVENTIONS.md](docs/CONVENTIONS.md): ids, translations, specs, code and assets conventions.
- [docs/HORIZON-ISSUES.md](docs/HORIZON-ISSUES.md): the Horizon issues we raised or follow, their status, and the
  workaround each one left in our code. Keep it up to date.
- [docs/MODELS.md](docs/MODELS.md): pictures of every model (`cargo creature preview` makes them).
- One `SPECS.md` per module, the source of truth for it, its §0 the current status (the library's:
  [common/SPECS.md](common/SPECS.md)).

## Repository

| Path | Contents |
|---|---|
| `common/` | The library, `creature`: `content/` (`creature.hmod`: the jar, the guide, the creative tab, translations), `native/` (`creature.hnmod`: the crate `mod-creature` in `api/`, and the creative tab injection `jar_tab.rs`), `SPECS.md` |
| `mods/<mod>/` | One mod: `content/` (its `.hmod`: entities, items, recipes, loot, spawns, textures, sounds, translations), `native/` (its `.hnmod`: the crate `mod-<mod>` in `api/`, any injection helper beside it), `assets/models/*.bbmodel` (Blockbench models), `assets/sounds.toml` (its sounds' sources), `module.toml`, `SPECS.md`, `README.md` (its page for players) |
| `tools/` | The build tools, `cargo creature <command>` |
| `docs/` | Series-wide documentation |
| `.github/workflows/ci.yml` | CI (below) |

Generated files are committed and checked by `cargo creature check`: the creative tab
(`common/content/data/creature/horizon/creative_tab/creature.json`), each model's Rust (`native/api/src/model*.rs`) and
the models' pictures (`docs/models/`, `docs/MODELS.md`). Never edit them by hand.

## Build and test

You need the [Horizon launcher](https://playhorizon.dev) with a test instance (here `creature-dev`, pinned to the
series' Horizon Nightly), Rust, `cargo horizon-mod` (from Horizon's sources) and ffmpeg to remake the sounds.

```sh
cargo install --locked --git https://github.com/horizon-rust/horizon cargo-horizon-mod
cargo creature install                                   # the library and every mod, in creature-dev
cargo creature install vulture --instance my-instance    # one mod (with the library and what it requires)
```

`install` writes the creative tab, builds each module's content (`.hmod`) and native (`.hnmod`) halves, installs them
in the instance and runs its native build (a minute or two, longer the first time). Then pick the instance in the
launcher and press **Play** (restart the launcher if it was open during the build).

The native mods read internal game fields and inject into Horizon's code, so they are **pinned** to one Horizon
revision, the same for every module (`[compatibility] horizon_revision`, `api/Cargo.toml`, CI's `HORIZON_REV`). Keep
your test instance on that Nightly: on a newer one the launcher tries to rebuild them against code they weren't written
for.

| Command | What it does |
|---|---|
| `cargo creature list` | The library and the mods, with their status |
| `cargo creature tab` | Writes the series' creative tab from the modules' `module.toml` |
| `cargo creature model <mod> [model]` | Converts a Blockbench model into Rust (`native/api/src/model.rs`, or `model_<model>.rs`) |
| `cargo creature preview` | Redraws every model's pictures for `docs/MODELS.md` |
| `cargo creature sounds <mod> [folder]` | Cuts a mod's `.ogg` files from freesound downloads (default `sounds-src/`), as its `assets/sounds.toml` says |
| `cargo creature pack <module>` | Packages a module's native half into `target/<id>.hnmod` |
| `cargo creature build [mod...]` | Builds the library and the mods into `target/dist/`, without installing |
| `cargo creature check` | The repository's consistency: tab, manifests, pins, translations, JSON, generated files, docs |
| `cargo creature native-check <horizon-source>` | Clippy on every native crate against a Horizon source (a checkout at the pinned revision) |
| `cargo creature install [--instance <name>] [mod...]` | Builds and installs, then runs the instance's native build |

## Before committing

- `rustfmt` on the tools and the native code (`rustfmt.toml`), Clippy with `-D warnings` on the tools and the native
  crates (`cargo creature native-check`), `cargo creature check`.
- After a native change: `cargo creature install` and a test in game. The injections (`native/*.rs` beside `api/`)
  compile only in the launcher's build, never in CI.
- After a model change: `cargo creature model <mod> [model]` and `cargo creature preview`.
- Everything in the repository is in English. The game text is translated: keep each module's
  `assets/<id>/lang/en_us.json` and `fr_fr.json` in step. No em dash anywhere.
- A change to a module updates its `SPECS.md` §0 (the current status, not a log) and, when players see it, its
  `README.md`.

CI (`.github/workflows/ci.yml`) runs all of this on every push and pull request, then builds every module and keeps the
files as an artifact.

## Rules of the series

- **The rule of two**: a system used by two or more mods goes in the library; species content stays in its mod. Never
  duplicate a system between mods.
- **No mod requires another**: optional content between mods goes through conditional recipes
  (`neoforge:conditions`, `mod_loaded`, `not`). A library feature some mods need (the glass jar) switches on with them.
- **One creative tab**, the library's, generated from each module's `module.toml` (`creative` list). Mods declare none.
- **Horizon issues**: a missing API or a bug becomes an issue (modding issues on
  [arismoko/horizon](https://github.com/arismoko/horizon/issues), game bugs on
  [horizon-rust/horizon](https://github.com/horizon-rust/horizon/issues)) and a workaround marked in the code with its
  issue, listed in `docs/HORIZON-ISSUES.md`.

Adding a mod: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), "Adding a mod".
