# Horizon Creature: creature mods for Minecraft Horizon

A series of creature mods for [Minecraft Horizon](https://playhorizon.dev), the Rust reimplementation of Minecraft Java
26.3. Each mod is independent and requires one shared library, **Horizon Creature**. All their items share one creative
tab, "Horizon Creature". The game text is in English and French.

| Mod | What it adds | Status |
|---|---|---|
| [Horizon Creature](common/README.md) (`creature`) | The library: shared systems (incubator, glass jar, riding, taming…) and the creative tab. **Required by every mod.** | prototype |
| [Vulture](mods/vulture/README.md) (`vulture`) | A vulture you tame, ride and fly, with chests and talons | prototype (rideable, flies) |
| [Butterflies](mods/butterflies/README.md) (`butterflies`) | Butterflies in many colours, caught in a glass jar and settled in your garden | planned |
| [Wyverns](mods/wyverns/README.md) (`wyverns`) | A dark mountainous dimension of harpies and wyverns, the Harpy Queen boss, wyverns hatched and ridden. **Also requires Butterflies.** | planned |
| [Snakes](mods/snakes/README.md) (`snakes`) | Aggressive snakes in many colours, whose eggs hatch into named pets | planned |
| [Worms](mods/worms/README.md) (`worms`) | Earthworms you carry in a jar to your garden, where they help crops grow | planned |

> Single player only for now; multiplayer comes later for the whole series.

## Documentation

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): the modules, their dependencies and responsibilities, what goes in the
  library, the creative tab, building, adding a mod.
- [docs/CONVENTIONS.md](docs/CONVENTIONS.md): ids, translations, specs, code and assets conventions.
- One `SPECS.md` per module (the library's: [common/SPECS.md](common/SPECS.md)), with its progress in §0.

## Install and test

You need the [Horizon launcher](https://playhorizon.dev) with a test instance (here `creature-dev`), Rust (with
`cargo horizon-mod`, from Horizon's sources); ffmpeg to remake the sounds.

```sh
cargo install --locked --git https://github.com/horizon-rust/horizon cargo-horizon-mod
cargo creature install                          # the library and every mod that has something to build
cargo creature install vulture --instance my-instance   # a mod, with the mods it requires
```

It writes the shared creative tab, builds the content half (`.hmod`) and the native half (`.hnmod`) of the library and
of each mod, installs them in the instance and runs its native build (a minute or two, longer the first time). Then, in
the launcher, pick the instance and press **Play** (restart the launcher if it was open during the build). The native
mods are pinned to one Horizon revision, because they read internal game fields.

The tools (`tools/`, Rust, `cargo creature <command>`):

| Command | What it does |
|---|---|
| `cargo creature list` | The library and the mods, with their status |
| `cargo creature tab` | Writes the series' creative tab from the modules' `module.toml` |
| `cargo creature model <mod> [model]` | Converts a mod's Blockbench model into Rust (`native/api/src/model.rs`) |
| `cargo creature sounds <mod> [folder]` | Makes a mod's `.ogg` files from freesound downloads (vulture) |
| `cargo creature pack <module>` | Packages a module's native half into `target/<id>.hnmod` |
| `cargo creature build [mod...]` | Builds the library and the mods into `target/dist/`, without installing |
| `cargo creature check` | Checks the repository's consistency (also run by CI) |
| `cargo creature install [--instance <name>] [mod...]` | Builds and installs the library and the mods, then runs the native build |

## Repository

| Path | Contents |
|---|---|
| `common/` | The library (content, native, spec) |
| `mods/<mod>/` | One folder per mod: `content/`, `native/`, `assets/models/`, `SPECS.md`, `README.md`, `module.toml` |
| `tools/` | The build tools |
| `docs/` | Series-wide documentation |
| `.github/workflows/ci.yml` | CI: format, lint, tests, `cargo creature check`, build with the packages as an artifact |
