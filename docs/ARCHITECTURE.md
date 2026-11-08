# Architecture

The repository holds a **series of mods** for [Minecraft Horizon](https://playhorizon.dev) (a Rust reimplementation of
Minecraft Java 26.3), called **Horizon Creature**: one mandatory library and independent creature mods that all depend
on it.

## 1. Modules

| Module | Folder | Mod id (namespace) | Status | Spec |
|---|---|---|---|---|
| **Horizon Creature** (library) | `common/` | `creature` | prototype | [common/SPECS.md](../common/SPECS.md) |
| Vulture | `mods/vulture/` | `vulture` | prototype | [mods/vulture/SPECS.md](../mods/vulture/SPECS.md) |
| Butterflies | `mods/butterflies/` | `butterflies` | planned (spec) | [mods/butterflies/SPECS.md](../mods/butterflies/SPECS.md) |
| Wyverns | `mods/wyverns/` | `wyverns` | planned (spec) | [mods/wyverns/SPECS.md](../mods/wyverns/SPECS.md) |
| Snakes | `mods/snakes/` | `snakes` | planned (spec) | [mods/snakes/SPECS.md](../mods/snakes/SPECS.md) |
| Worms | `mods/worms/` | `worms` | planned (spec) | [mods/worms/SPECS.md](../mods/worms/SPECS.md) |

`cargo creature list` prints the same table from the modules' `module.toml`.

## 2. Dependencies

```
                    ┌──────────────────────────────┐
                    │  Horizon (pinned revision)   │
                    └──────────────▲───────────────┘
                                   │
                    ┌──────────────┴───────────────┐
                    │ Horizon Creature (`creature`)│  library: shared systems, the series' creative tab
                    └──▲──────▲──────▲──────▲───▲──┘
                       │      │      │      │   │      required dependency, never the other way round
                 vulture  butterflies◄─wyverns snakes worms
                              (wyverns requires butterflies: its staff is crafted with butterfly dust)
```

- Every mod **requires** the library, and the library never depends on a mod (it does not even know their ids, except
  in the generated creative tab and in the jar recipe's condition, below).
- Mods are **independent by default**: any subset can be installed with the library. A mod-to-mod requirement is the
  exception, decided in the specs, never circular, and declared everywhere it matters (below). Today there is one:
  **Wyverns requires Butterflies** ✅ (the dimension staff needs `butterflies:butterfly_dust`).
- A *system* needed by two mods is never a reason for one to require the other: it belongs in the library (§3). Only
  *content* (an item used in a recipe) justifies a mod-to-mod requirement.
- **Optional** features that switch on with a mod are the library's: e.g. the glass jar (common/SPECS.md §5) is only
  craftable when Butterflies or Worms is on, through a conditional recipe (`neoforge:conditions` / `mod_loaded`, which
  Horizon supports).

How the dependency is expressed, per half of a mod (Horizon has two kinds of mod, and our mods are both):

| Half | File | Dependency mechanism |
|---|---|---|
| Native (`.hnmod`, Rust compiled into the game) | `native/horizon-mixin.toml` | `[dependencies] creature = "^0.1"` (plus `butterflies = "^0.1"` for Wyverns). The launcher refuses to build without it, orders the hooks (library first) and gives the mod's `[api]` crate a `creature` crate to call (Horizon's `docs/native-mods.md`, "Dependencies between trait mods"). |
| Content (`.hmod`, data and assets) | `content/horizon-mod.toml` | None exists in Horizon's content manifest. The native dependency covers it in practice (the library's native half is installed with its content half); the description says "Needs Horizon Creature" (and the other mods required). |
| Repository tools | `module.toml` | `requires = ["butterflies"]`: `cargo creature install wyverns` also installs Butterflies, first. |

A mod with no native half yet (a pure content mod) has no enforced dependency: that is acceptable while it is in
development, and every mod of the series is expected to have a native half anyway.

## 3. What goes where

**The rule of two.** A system goes in the library when **two or more mods use it or are specified to use it**, or when
it is an integration point of the series (the creative tab). Otherwise it stays in its mod, and moves to the library the
day a second mod needs it (the move is part of that second mod's work, not a copy).

| In the library (`creature`) | In a mod |
|---|---|
| Generic systems, configured by data or by Rust parameters: incubator, glass jar and territories, riding and flight, stamina, taming and owners, naming, variants and patterns, breeding and growth, model drawing, mob sounds | The species: entity types, models, textures, sounds, items, recipes, loot, spawns, AI tuning, balance numbers |
| Shared blocks and items (`creature:incubator`, `creature:glass_jar`) | The mod's own blocks and items, eggs included (`wyverns:fire_wyvern_egg`) |
| The series' creative tab | The list of its items for that tab (`module.toml`) |
| Shared translations (`creature` lang files) | Its own translations |

The full list of shared systems, done and planned, is in [common/SPECS.md](../common/SPECS.md) §3 to §5.

**API shape.** A mod uses the library by calling it from its own hooks (`creature::model::draw(...)`), or by
registering its species with it at start (`creature::incubator::register(...)`, planned): the mod's `hooks()` runs after
the library's, in dependency order. The library publishes plain functions, types and statics; its state is behind
`Mutex`es, since hooks can be reentrant.

**Mixin injections** (code injected into Horizon's own functions, such as the vulture's right-click mounting) are compiled
into the game, and two mods injecting the same function each add their own copy. An injection that several mods would
need moves to the library once, with a list of the kinds it applies to that the mods fill in.

## 4. Creative inventory

The whole series shares **one creative tab**, "Horizon Creature", owned by the library
(`common/content/data/creature/horizon/creative_tab/creature.json`).

- Horizon does not let a content mod add items to another mod's tab, but a tab may name items of other mods, and the
  game leaves out those nobody registers. So the library's tab lists the items of **every** mod of the series; with only
  some mods installed, only their items show.
- The file is **generated** by `cargo creature tab` (and by every `install`): the library's items first
  (`common/module.toml`), then each mod's (`creative` in `mods/<mod>/module.toml`), mods in folder-name order. Never edit
  it by hand.
- A mod therefore declares **no tab of its own**: its items reach the tab by being listed in its `module.toml`.
- The tab's button shows `common/module.toml`'s `icon` (a vanilla egg until `creature:incubator` exists).
- A new item in a mod changes the library's file: release the library with the mod (§6).
- Every mod item also appears in the search tab under its name (Horizon does it).

## 5. Repository layout

```
.cargo/config.toml      `cargo creature <command>` alias
docs/                   series-wide docs: ARCHITECTURE.md (this file), CONVENTIONS.md
common/                 the library
  module.toml           repository metadata for the tools (id, name, status, tab icon, its tab items)
  SPECS.md, README.md
  content/              content half: horizon-mod.toml, data/creature/, assets/creature/
  native/               native half: horizon-mixin.toml, api/ (crate `mod-creature`, reached as `creature`)
mods/<mod>/             one folder per mod, all alike:
  module.toml           (`requires` lists the other mods of the series it needs, if any)
  SPECS.md, README.md
  content/              content half (id = the mod's id); absent while the mod is only specified
  native/               native half: horizon-mixin.toml (with `[dependencies] creature`), api/, injections
  assets/models/        Blockbench sources (`<model>.bbmodel`)
tools/                  the Rust build tools (`cargo creature ...`), shared by every module
```

## 6. Build, install and versions

| Command | What it does |
|---|---|
| `cargo creature list` | The modules, their status and which halves they have |
| `cargo creature tab` | Writes the shared creative tab |
| `cargo creature model <mod> [model]` | `mods/<mod>/assets/models/<model>.bbmodel` → `native/api/src/model.rs` (`model_<model>.rs` for a model not named after the mod) |
| `cargo creature sounds <mod> [folder]` | Makes the mod's `.ogg` files from freesound downloads (the vulture's only, for now) |
| `cargo creature pack <module>` | A module's `native/` → `target/<id>.hnmod` |
| `cargo creature install [--instance <name>] [mod...]` | Writes the tab, builds and installs the library and the mods (default: every mod with something to build) in the instance (default `vulture-dev`), turns their content on and runs the instance's native build |

- **One Horizon revision for the whole series**: every `horizon-mixin.toml` and `api/Cargo.toml` pins the same commit.
  Moving to a new revision is done for all modules at once.
- **Versions** are semver, per module (`[mod] version` in `horizon-mixin.toml` and `version` in `horizon-mod.toml`, kept
  equal). Mods require the library by `^<major>.<minor>` while it is `0.x`. A breaking change in the library bumps its
  minor version (0.x) and every mod's requirement with it.
- A mod's content and native halves are released together, with the library version they were tested with.

## 7. Adding a mod

1. `mods/<mod>/` with `module.toml` (`status = "planned"`), `SPECS.md` (from an existing one: §0 progress, ✅/❓) and
   `README.md`. Add it to the table in §1 and to the root README.
2. Write the spec. For each system it needs, check [common/SPECS.md](../common/SPECS.md): reuse what exists, mark what
   should move to the library (rule of two, §3), and add the library's side of the work to the library's spec.
3. When implementation starts: `content/horizon-mod.toml` (id = the folder name, `side = "both"`, no creative tab),
   `native/horizon-mixin.toml` (same Horizon pin, `[dependencies] creature = "^0.1"` and any mod it requires,
   `[api] path = "api"`),
   `native/api/` (a crate `mod-<mod>` with `pub fn hooks()`), `en_us` and `fr_fr` lang files, items listed in
   `module.toml`.
4. `cargo creature install <mod>`.

Nothing else in the repository changes: the tools find modules by their `module.toml`.
