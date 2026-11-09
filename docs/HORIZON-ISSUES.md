# Horizon issues

The issues this project raised (or follows) on Horizon, and the workaround each one left in our code. Update it when an
issue moves, and before moving the series to a new Horizon revision (docs/CONVENTIONS.md, "Versions and commits").

Where things go, since 2026-11-09: **pull requests** about the mod API, native hooks, native, content and wasm mods,
mixins and `cargo horizon-mod` go to [arismoko/horizon](https://github.com/arismoko/horizon) (board:
[arismoko project 1](https://github.com/users/arismoko/projects/1)); the **issues** stay open on
[horizon-rust/horizon](https://github.com/horizon-rust/horizon), and claims on them stand. Everything else (gameplay,
rendering, launcher) stays on horizon-rust/horizon. **New** modding issues (proposals, API gaps, mod-loading bugs) go
to [arismoko/horizon's issues](https://github.com/arismoko/horizon/issues) (its README); game bugs unrelated to modding
go to horizon-rust/horizon. Before opening an issue, search both repositories.

Status: **to open** (found, not filed), **open** (filed, nothing merged), **claimed** (someone works on it), **fixed
upstream** (merged, not yet in our pinned revision), **done** (in our pinned revision, workaround removed),
**closed** (won't change, or not a bug).

Our pinned revision: `fdc87ae` (every `horizon-mixin.toml`, `api/Cargo.toml` and CI's `HORIZON_REV`).

## Open

| Issue | What | Status (2026-11-09) | Our workaround (where) | Drop it when fixed |
|---|---|---|---|---|
| [#13971](https://github.com/horizon-rust/horizon/issues/13971) | A mod left out for an invalid injection helper still lets `native-build` exit 0 ("made for another Horizon version") | claimed (iseaprojects), confirmed | `cargo creature install` reads `build.log` and fails on "was left out" (`tools/src/install.rs`, `left_out`) | Keep as a safety net; simplify once the build fails itself |
| [#13972](https://github.com/horizon-rust/horizon/issues/13972) | An injection into `native_interact_entity` sees the client's mob copies (changes lost, `start_riding` client-only); undocumented | open, documentation (cause confirmed: the window's `Game` is a client of the server thread) | Clicks in the `interact_entity` trait hook, mounting with `Host::mount` (`mods/vulture/native/api/src/interaction.rs`) | Nothing to drop: the trait hook is the right path |
| [#13973](https://github.com/horizon-rust/horizon/issues/13973) | `Mob.extra.saddled` on a mod mob makes Horizon drive it as a steered mount | claimed (RLC248): an engine parity bug in `equine::controlled()` (`_ => m.extra.saddled` for every species) | The harness is kept in the vulture's state, never in `saddled`, which is forced to false (`mods/vulture/native/api/src/state.rs`) | Once fixed, `saddled` could hold the harness again (not needed) |
| [#13974](https://github.com/horizon-rust/horizon/issues/13974) | `open_menu` returns `None` from the `interact_entity` hook | claimed (iseaprojects): menus will be queued and opened when the turn ends | The vulture's screen is opened on its next tick (`mods/vulture/native/api/src/screen.rs`, `ask` / `tick`) | Open it directly from the click |
| [#13975](https://github.com/horizon-rust/horizon/issues/13975) | No way to set the passenger seat of a content mod entity type | claimed (oloion): `passenger_attachments` in `entity_type` JSON | Hitbox height 0.6 so the rider sits on the harness (`mods/vulture/native/api/src/lib.rs`, `HEIGHT`) | Height back to ~1.0, seat declared in `content/data/vulture/horizon/entity_type/vulture.json` |
| [#6276](https://github.com/horizon-rust/horizon/issues/6276) (not ours; [our comment](https://github.com/horizon-rust/horizon/issues/6276#issuecomment-6743829717)) | An entity's size (design needed); we asked for a static size in `entity_type` JSON | open, design needed | Each vulture gets a cloned `MobType` with its own width and height in `register_goals` (`mods/vulture/native/api/src/lib.rs`) | Size declared in `vulture.json` |

## Closed

| Issue | What | Outcome |
|---|---|---|
| [#13976](https://github.com/horizon-rust/horizon/issues/13976) | "Mod entity types from wandering_trader can't be babies or age" | Not a bug, our mistake (2026-11-09): the trader is in both lists. Our own age counter was removed (it aged vultures twice). |

## To open (candidates)

Found while developing, not filed yet. File one only with a clean repro.

- **`Host::take_items` takes nothing from a non-plain stack** (an item from the creative tab may carry component data):
  a "take one from the hand that clicked" helper, or a doc note. We take items only when the player isn't in creative.
- **Mod keys only appear on the mod's page in Options > Mods**, not in Controls, where players look for them.
- **Suspected, unconfirmed:** an entity's attachment not readable on its first tick after loading (our "untamed one
  load in four"; we now read again next tick). Needs a repro before filing.
- **Suspected, unconfirmed:** `/summon` with `horizon:attachments` NBT doesn't set the attachments (maybe our format).

## Coming with the new mod system

Announced 2026-11-09 (not in our pinned revision yet), to plan the migration when we move to a newer revision:

- **Prebuilt game:** the launcher compiles only the mod (~1 s) and links it against a prebuilt game, no game rebuild.
- **`Hooks` split into `ServerHost` / `ClientHost`** (~260 hooks): our `&mut dyn Host` hooks and helpers get split by
  side; a client call in server code no longer compiles.
- **Parts** (`ItemPart`, `BlockPart`, `EntityPart` coming): behaviour attached to an item, block or entity, like
  overriding `Item` in Java. Fits the harness, the incubator, the glass jar.
- **`[[inject]]` in `horizon-mod.toml`** replaces `[[injection]]` in `horizon-mixin.toml`: `access = "api"` (host and
  copied parameters, native and wasm, no rebuild) or `access = "internals"` (the real `Game`, native only; a pinned
  fingerprint refuses a changed target). Our two remaining injections (`mount_hud.rs`, `ride_keys.rs`) move there.
- **Mods extending mods:** plain Rust dependencies between mods already work (our `creature` crate); coming: a library
  declaring its own hooks, Part kinds, services, registries, and injection points into other mods. The library could
  then publish events (a creature hatched, a creature caught in a jar) that mods listen to.
- **Dimensions (NeoForge-style) are in:** dimension types, biome sources, sky / fog / clouds, per-level sea level and
  clocks, entity teleport. The main risk of Wyverns (a custom dimension, `mods/wyverns/SPECS.md` §1.1) should be
  removable on the new revision.
