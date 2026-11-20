# Architecture

The repository holds a **series of mods** for [Sunrise](https://playsunrise.net) (Minecraft Java 26.3 rewritten in
Rust, formerly Horizon), called **Sunrise Creatures**: one mandatory library and independent creature mods that all depend
on it.

## 1. Modules

| Module | Folder | Mod id (namespace) | Status | Spec |
|---|---|---|---|---|
| **Sunrise Creatures** (library) | `common/` | `creature` | playable | [common/SPECS.md](../common/SPECS.md) |
| Vulture | `mods/vulture/` | `vulture` | playable | [mods/vulture/SPECS.md](../mods/vulture/SPECS.md) |
| Butterflies | `mods/butterflies/` | `butterflies` | playable | [mods/butterflies/SPECS.md](../mods/butterflies/SPECS.md) |
| Worms | `mods/worms/` | `worms` | playable | [mods/worms/SPECS.md](../mods/worms/SPECS.md) |
| Cyclops | `mods/cyclops/` | `cyclops` | playable | [mods/cyclops/SPECS.md](../mods/cyclops/SPECS.md) |
| Fireflies | `mods/fireflies/` | `fireflies` | playable | [mods/fireflies/SPECS.md](../mods/fireflies/SPECS.md) |
| Dung Beetles | `mods/dung_beetles/` | `dung_beetles` | planned | [mods/dung_beetles/SPECS.md](../mods/dung_beetles/SPECS.md) |
| Sporelins | `mods/sporelins/` | `sporelins` | planned | [mods/sporelins/SPECS.md](../mods/sporelins/SPECS.md) |
| Living Flowers | `mods/living_flowers/` | `living_flowers` | planned | [mods/living_flowers/SPECS.md](../mods/living_flowers/SPECS.md) |
| Storm Spirits | `mods/storm_spirits/` | `storm_spirits` | playable | [mods/storm_spirits/SPECS.md](../mods/storm_spirits/SPECS.md) |
| Jellyfish | `mods/jellyfish/` | `jellyfish` | playable | [mods/jellyfish/SPECS.md](../mods/jellyfish/SPECS.md) |
| Magma Golem | `mods/magma_golem/` | `magma_golem` | planned | [mods/magma_golem/SPECS.md](../mods/magma_golem/SPECS.md) |
| Fairies | `mods/fairies/` | `fairies` | playable | [mods/fairies/SPECS.md](../mods/fairies/SPECS.md) |
| Seagulls | `mods/seagulls/` | `seagulls` | planned | [mods/seagulls/SPECS.md](../mods/seagulls/SPECS.md) |
| Flying Fish | `mods/flying_fish/` | `flying_fish` | planned | [mods/flying_fish/SPECS.md](../mods/flying_fish/SPECS.md) |
| Phasmids | `mods/phasmids/` | `phasmids` | planned | [mods/phasmids/SPECS.md](../mods/phasmids/SPECS.md) |
| Scorpions | `mods/scorpions/` | `scorpions` | playable | [mods/scorpions/SPECS.md](../mods/scorpions/SPECS.md) |
| Vanilla Bestiary | `mods/vanilla_bestiary/` | `vanilla_bestiary` | playable | [mods/vanilla_bestiary/SPECS.md](../mods/vanilla_bestiary/SPECS.md) |

`cargo creature list` prints the same table from the modules' `module.toml`.

## 2. Dependencies

```
                    ┌──────────────────────────────────────┐
                    │      Sunrise (pinned build)          │
                    └───────────────────▲──────────────────┘
                                        │
                    ┌───────────────────┴──────────────────┐
                    │    Sunrise Creatures (`creature`)     │  library: shared systems, the series' creative tab
                    └───▲───────▲───────▲───────▲───────▲──┘
                        │       │       │       │       │      required dependency, never the other way round
                     vulture    │     worms     │    cyclops
                           butterflies      fireflies

                        No mod requires another.
```

- Every mod **requires** the library, and the library never depends on a mod (it does not even know their ids, except
  in the generated creative tab and in the jar recipe's condition, below).
- Mods are **independent**: any subset can be installed with the library. A mod-to-mod requirement would be the
  exception, decided in the specs, never circular, and declared everywhere it matters (below). **Today there is none**
  ✅.
- **Optional content** between mods goes through conditional recipes, never a requirement: a recipe using another
  mod's item has one file per combination (`neoforge:conditions` with `mod_loaded`, `not`).
- A *system* needed by two mods is never a reason for one to require the other: it belongs in the library (§3). Only
  *content* (an item used in a recipe) justifies a mod-to-mod requirement.
- **Optional** features that switch on with a mod are the library's: e.g. the glass jar (common/SPECS.md §4) is only
  craftable, and only shown, when Butterflies, Worms or Fireflies is on, through a conditional recipe
  (`neoforge:conditions` / `mod_loaded`, which Sunrise supports).

How the dependency is expressed:

| Where | File | Dependency mechanism |
|---|---|---|
| The package (`.smod`: content and Rust) | `sunrise-mod.toml` | `[dependencies] creature = "^0.3"` (plus any mod it requires). The game refuses to load without it and runs the library's hooks first; the launcher gives the mod's crate a `creature` crate to call (Sunrise's modding book, "Dependencies between mods"). |
| Repository tools | `module.toml` | `requires = ["<mod>"]` (none today): `cargo creature install <mod>` then also installs the mods it requires, first. |


## 3. What goes where

**The rule of two.** A system goes in the library when **two or more mods use it or are specified to use it**, or when
it is an integration point of the series (the creative tab). Otherwise it stays in its mod, and moves to the library the
day a second mod needs it (the move is part of that second mod's work, not a copy).

| In the library (`creature`) | In a mod |
|---|---|
| Generic systems, configured by data or by Rust parameters: glass jar and territories, the small fliers' flight and spawning, friends, colours and patterns, model drawing, glowing creatures' lights, fighting back, water creatures, touches, hunters, mob sounds, monsters sparing our creatures, the bestiary | The species: entity types, models, textures, sounds, items, recipes, loot, spawns, AI tuning, balance numbers |
| Shared blocks and items (`creature:glass_jar`, `creature:guide`) | The mod's own blocks and items (`cyclops:club`) |
| The series' creative tab | The list of its items for that tab (`module.toml`) |
| Shared translations (`creature` lang files) | Its own translations |

The full list of shared systems, done and planned, is in [common/SPECS.md](../common/SPECS.md) §3 to §7.

**API shape.** A mod uses the library by calling it from its own hooks (`creature::model::draw(...)`), or by
registering its species with it at start (`creature::jar::register(...)`, `creature::bestiary::register(...)`): the mod's `hooks()` runs after
the library's, in dependency order. The library publishes plain functions, types and statics; its state is behind
`Mutex`es, since hooks can be reentrant.

**Mixin injections** (code injected into the game's own functions, `[[native.mixin]]`) are compiled
into the game, and two mods injecting the same function each add their own copy. An injection that several mods would
need moves to the library once, with a list of the kinds it applies to that the mods fill in.

## 4. Creative inventory

The whole series shares **one creative tab**, "Sunrise Creatures", owned by the library
(`common/data/creature/sunrise/creative_tab/creature.json`).

- Sunrise does not let a content mod add items to another mod's tab, but a tab may name items of other mods, and the
  game leaves out those nobody registers. So the library's tab lists the items of **every** mod of the series; with only
  some mods installed, only their items show.
- The file is **generated** by `cargo creature tab` (and by every `install`): every spawn egg first, then the library's
  items (`common/module.toml`), then each mod's other items (`creative` in `mods/<mod>/module.toml`); mods in alphabetical
  (folder-name) order, each one's items in its own order. Never edit it by hand.
- A mod therefore declares **no tab of its own**: its items reach the tab by being listed in its `module.toml`.
- The tab's button shows `common/module.toml`'s `icon`: the creature guide book, `creature:guide`.
- A new item in a mod changes the library's file: release the library with the mod (§6).
- Every mod item also appears in the search tab under its name (Sunrise does it).

## 5. Repository layout

```
.cargo/config.toml      `cargo creature <command>` alias
docs/                   series-wide docs: ARCHITECTURE.md (this file), CONVENTIONS.md, SUNRISE-ISSUES.md,
                        PUBLISHING.md; models/ (the models' pictures, generated)
registry/               the published modules' registry entries and icons, as sunrise-mods holds them (written by
                        `cargo creature release`)
common/                 the library
  module.toml           repository metadata for the tools (id, name, status, tab icon, its tab items, `ported`,
                        its registry page)
  CHANGELOG.md          its versions' changes (the registry's changelog)
  SPECS.md, README.md
  sunrise-mod.toml      the package (id, version, [native] pins); with Cargo.toml (crate `mod-creature`, reached as
  Cargo.toml, src/      `creature`), src/, assets/creature/, data/creature/ (blocks, items, tab in data/creature/sunrise/)
  assets/, data/
mods/<mod>/             one folder per mod, all alike:
  module.toml           (`requires` lists the other mods of the series it needs, if any)
  SPECS.md, README.md
  sunrise-mod.toml ...  the package, as the library's ([dependencies] creature); absent while the mod is only specified
  sources/              not shipped: Blockbench models (`models/<model>.bbmodel`), `sounds.toml`, Horizon's old manifests
tools/                  the Rust build tools (`cargo creature ...`), shared by every module
```

## 6. Build, install and versions

| Command | What it does |
|---|---|
| `cargo creature list` | The modules, their status and what they have (content, native) |
| `cargo creature tab` | Writes the shared creative tab, and the library's table of every module's recipes (`common/src/recipes.rs`, the guide's Items tab) |
| `cargo creature model <mod> [model]` | `mods/<mod>/sources/models/<model>.bbmodel` → `src/model.rs` (`model_<model>.rs` for a model not named after the mod) |
| `cargo creature vanilla [jar]` | Minecraft's own data (natural spawns, loot tables, food tags) from the game's jar (default: the launcher's) → the Vanilla Bestiary's `src/game_data.rs` |
| `cargo creature sounds <mod> [folder]` | Makes the mod's `.ogg` files from freesound downloads |
| `cargo creature pack <module>` | A module's package → `target/stage/<kind>/<id>/` and `target/<id>.smod` |
| `cargo creature build [mod...]` | Writes the tab, packages and checks (`cargo sunrise-mod check`) the library and the mods (default: all) into `target/dist/`, without installing |
| `cargo creature check` | The repository's consistency (docs/CONVENTIONS.md, "Formatting, lint and checks") |
| `cargo creature native-check [sdk]` | Clippy and the tests of the ported crates against the pinned SDK |
| `cargo creature install [--instance <name>] [mod...]` | Writes the tab, packages and installs the library and the mods (default: every mod with a package) in the instance (default `creature-dev`): the ported ones installed in its `sunrise-native-mods/` and built by `sunrise-launcher native-build --instance`, the others' content in `sunrise-mods/`, all turned on |

- **One Sunrise build for the whole series**: every `sunrise-mod.toml` pins the same `[native] sunrise` and `mixins`
  commits, and so does CI (`SUNRISE_REV`). Moving to a new build (`cargo sunrise-mod update`) is done for all modules
  at once.
- **CI** (`.github/workflows/ci.yml`), on every push to `main` and pull request: rustfmt, Clippy and the tools' tests,
  `cargo creature check`, `cargo creature native-check`, then `cargo creature build` with the packages as an artifact.
  The game build itself happens in the launcher (`cargo creature install`).
- **Versions** are semver, per module (`version` in `sunrise-mod.toml` and in `Cargo.toml`, kept equal). Mods require
  the library by `^<major>.<minor>` while it is `0.x`. A breaking change in the library bumps its minor version (0.x)
  and every mod's requirement with it.
- A mod is released with the library version it was tested with.

## 7. Adding a mod

1. `mods/<mod>/` with `module.toml` (`status = "planned"`), `SPECS.md` (from an existing one: §0 progress, ✅/❓) and
   `README.md`. Add it to the table in §1 and to the root README.
2. Write the spec. For each system it needs, check [common/SPECS.md](../common/SPECS.md): reuse what exists, mark what
   should move to the library (rule of two, §3), and add the library's side of the work to the library's spec.
3. When implementation starts: `sunrise-mod.toml` (id = the folder name, `side = "both"`, `api = "0.2"`,
   `capabilities = ["registries"]`, the series' `[native]` pins, `[dependencies] creature = "^0.3"` and any mod it
   requires), `Cargo.toml` (a crate `mod-<mod>` on `sunrise-mod-api` from `.sunrise/sdk/`), `src/lib.rs` (its
   `#[mod_api::sunrise_mod] impl Hooks`), `assets/<mod>/lang/` `en_us` and `fr_fr`, `data/<mod>/sunrise/`, items listed in
   `module.toml`, `ported = true`.
4. `cargo creature install <mod>`.

Nothing else in the repository changes: the tools find modules by their `module.toml`.
