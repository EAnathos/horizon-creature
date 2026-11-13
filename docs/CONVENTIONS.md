# Development conventions

Rules shared by every module of the Horizon Creature series ([ARCHITECTURE.md](ARCHITECTURE.md)).

## Language

- Everything in the repository is in **English**: code, comments, docs, specs, commit messages.
- The game text is translated: every module keeps `assets/<id>/lang/en_us.json` and `fr_fr.json` in step (same keys).
- No em dash in the docs: use a colon, a semicolon, a comma or parentheses (`cargo creature check` refuses one).

## Ids and namespaces

- A module's mod id is its folder name, `[a-z0-9_]`, plural for a family of creatures (`butterflies`, `worms`) and
  singular for one species (`vulture`). The library is `creature`.
- A module only declares content in **its own namespace** (Horizon refuses two mods owning one namespace). Shared things
  live in `creature:`, species things in the mod's namespace: `creature:glass_jar`, `cyclops:cyclops_eye`.
- Names: `<species>` for an entity (`vulture:vulture`, `cyclops:cyclops`), `<species>_spawn_egg`, `<species>_egg`
  for a real egg, `<thing>` for an item. Entity texture paths: `<id>:entity/<species>/<variant>`.
- Lang keys follow vanilla: `entity.<id>.<name>`, `item.<id>.<name>`, `block.<id>.<name>`,
  `subtitles.entity.<id>.<sound>`. Only the library has an `itemGroup` key.
- Sounds: `<id>:entity.<species>.<sound>` in the module's `sounds.json`.
- Ids are permanent once a test world uses them: renaming breaks saves.

## Specs

- One `SPECS.md` per module, the source of truth for it; series-wide rules live in `docs/`. A mod's spec does not
  describe a library system: it links to `common/SPECS.md` and gives only its parameters (numbers, items, species).
- Markers: ✅ decided, ❓ proposal to confirm. Numbers are starting values, balanced in game.
- §0 is the progress: what is done (and tested in game), then what is left, in order. Dates are absolute
  (`2026-11-08`).
- The last section lists the open questions, struck through once answered with the answer.

## Code

- Native code follows the vulture's style: module docs (`//!`) explain the mechanism and link the spec section, Horizon
  internals that are relied on are named. Rust 1.99.1 (pinned by the launcher), edition 2024.
- A native crate may only use crates in Horizon's `Cargo.lock`. The tools (`tools/`) are a separate crate with their own
  lock.
- Server logic stays apart from rendering (multiplayer comes later).
- A mixin injection's helper file holds exactly one private function, named `<mod>_<injection id>` (helpers it
  needs go inside it); the launcher leaves the whole mod out otherwise, which `cargo creature install` reports.
- Generated files say so in their first line and are never edited by hand: `model.rs` (from `.bbmodel`), the shared
  creative tab.
- Before adding a system to a mod, check the library; before writing one that a second mod will need, put it in the
  library (ARCHITECTURE.md §3).

## Formatting, lint and checks

All three run in CI (`.github/workflows/ci.yml`) on every push to `main` and every pull request; run them before
committing:

```sh
cargo fmt --manifest-path tools/Cargo.toml                                   # format the tools
find common mods -path '*/native/*' -name '*.rs' -print0 | xargs -0 rustfmt --edition 2024   # format the native code
cargo clippy --manifest-path tools/Cargo.toml --all-targets -- -D warnings   # lint the tools
cargo creature check                                                         # the repository's consistency
```

- **rustfmt** with `rustfmt.toml` (edition 2024, 120 columns, as the rest of the repository). Generated models carry
  `#[rustfmt::skip]`.
- **Clippy** with warnings as errors, on the tools. The native crates can't be compiled outside the launcher (they need
  Horizon's lock file and the `creature` crate the launcher injects), so they are formatted in CI but compiled only by
  `cargo creature install`: run it before pushing native changes.
- **`cargo creature check`**: the shared tab is up to date, manifests carry the module's id and one version, every pin
  is the series' Horizon revision, mods depend on the library and on what they require, tab items exist, JSON parses,
  `en_us` and `fr_fr` have the same keys, no em dash in the docs.
- CI then builds every module (`cargo creature build`) and keeps the `.hmod` and `.hnmod` files as an artifact.

## Assets

- Models: Blockbench, "free" format, box UV, one root group, no rotated cubes (rotate groups). 64×64 textures unless a
  larger mob needs more.
- Sounds: CC0 only (freesound.org, licence checked), sources listed in the tool that makes them.

## Versions and commits

- Semver per module, content and native versions kept equal (ARCHITECTURE.md §6). One Horizon pin for the series.
- A commit touches one module when it can; a library change and the mod changes that need it go together.
