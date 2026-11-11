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

Status: **to open** (found, not filed), **open** (filed, nothing merged), **claimed** (someone works on it), **fixed on
the fork** (merged on arismoko/horizon, not yet in a Horizon release players get), **fixed upstream** (in a release,
not yet in our pinned revision), **done** (in our pinned revision, workaround removed),
**closed** (won't change, or not a bug).

Last checked: 2026-11-11 (pinned `fdc87ae`; arismoko/horizon's main; none of the fork's fixes is in a horizon-rust
release yet). Our pinned revision: `fdc87ae` (the newest release the launcher offers, nightly 2026-11-08) (every
`horizon-mixin.toml`, `api/Cargo.toml` and CI's `HORIZON_REV`).

## Open

| Issue | What | Status (2026-11-11) | Our workaround (where) | Drop it when fixed |
|---|---|---|---|---|
| [#13971](https://github.com/horizon-rust/horizon/issues/13971) | A mod left out for an invalid injection helper still lets `native-build` exit 0 ("made for another Horizon version") | open, no pull request (claimed by iseaprojects before the move to the fork) | `cargo creature install` reads `build.log` and fails on "was left out" (`tools/src/install.rs`, `left_out`) | Keep as a safety net; simplify once the build fails itself |
| [#13972](https://github.com/horizon-rust/horizon/issues/13972) | An injection into `native_interact_entity` sees the client's mob copies (changes lost, `start_riding` client-only); undocumented | open, documentation, no pull request | Clicks in the `interact_entity` trait hook, mounting with `Host::mount` (`mods/vulture/native/api/src/interaction.rs`) | Nothing to drop: the trait hook is the right path |
| [#13973](https://github.com/horizon-rust/horizon/issues/13973) | `Mob.extra.saddled` on a mod mob makes Horizon drive it as a steered mount | **fixed on the fork** (2026-11-09, [arismoko/horizon#2540](https://github.com/arismoko/horizon/pull/2540): only a horse or camel takes a saddled player rider's input); the issue stays open until reported back | The harness is kept in the vulture's state, never in `saddled`, which is forced to false (`mods/vulture/native/api/src/state.rs`) | Once fixed, `saddled` could hold the harness again (not needed) |
| [#13974](https://github.com/horizon-rust/horizon/issues/13974) | `open_menu` returns `None` from the `interact_entity` hook | open, no pull request (claimed by iseaprojects before the move: menus queued and opened when the turn ends) | The vulture's screen is opened on its next tick (`mods/vulture/native/api/src/screen.rs`, `ask` / `tick`) | Open it directly from the click |
| [#13975](https://github.com/horizon-rust/horizon/issues/13975) | No way to set the passenger seat of a content mod entity type | **fixed on the fork** (2026-11-09, [arismoko/horizon#2539](https://github.com/arismoko/horizon/pull/2539): `passenger_attachments` in `entity_type` JSON) | Hitbox height 0.6 so the rider sits on the harness (`mods/vulture/native/api/src/lib.rs`, `HEIGHT`) | Height back to ~1.0, seat declared in `content/data/vulture/horizon/entity_type/vulture.json` |
| [#6276](https://github.com/horizon-rust/horizon/issues/6276) (not ours; [our comment](https://github.com/horizon-rust/horizon/issues/6276#issuecomment-6743829717)) | An entity's size | open upstream; **on the fork**: an `entity_size` hook, NeoForge's `EntityEvent.Size` ([arismoko/horizon#1794](https://github.com/arismoko/horizon/pull/1794), [#832](https://github.com/arismoko/horizon/issues/832) closed), not a static size in JSON | Each vulture gets a cloned `MobType` with its own width and height in `register_goals` (`mods/vulture/native/api/src/lib.rs`) | Answer `entity_size` with the vulture's size |
| [arismoko#2738](https://github.com/arismoko/horizon/issues/2738) | A content mod's living type plays its template's sounds; `MobType.sounds` are always `minecraft:` | **fixed on the fork** (2026-11-11, [#2838](https://github.com/arismoko/horizon/pull/2838): optional `ambient`, `hurt`, `death`, `step` in a living `entity_type`, a full id or `"none"`) | Our mobs are muted and play their own sounds with `Host::play_sound` (`common/native/api/src/sounds.rs`) | Declare the sounds in `vulture.json`, drop the queue and the muting (the flaps and the eating stay ours) |
| [arismoko#2739](https://github.com/arismoko/horizon/issues/2739) | A mount can't read its rider's sneak: `tick_mount_riding` dismounts before copying the input, `Rider` has no sneak | **fixed on the fork** (2026-11-11, [#2986](https://github.com/arismoko/horizon/pull/2986): a dismount kept by `entity_mount` still gives the mount that tick's input, and `Rider` says sneak is held) | Injection at the head of `tick_mount_riding`, sneak passed through `rider.sprint` (`mods/vulture/native/ride_keys.rs`) | Cancel the dismount in `entity_mount` in the air and read the rider's sneak; drop `ride_keys.rs` |
| [arismoko#2740](https://github.com/arismoko/horizon/issues/2740) | `mob_submit` can't read a mob's synced attachments (`&dyn Host`, `get` needs `&mut`) | **fixed on the fork** (2026-11-10, [#2874](https://github.com/arismoko/horizon/pull/2874): `host.query(&TYPE)?.get(holder)` on the read-only snapshot query) | Render state in statics keyed by entity id (`state.rs`, `Look`; `flight.rs`, `FLAPPING`), single player only | A synced attachment read in `mob_submit` |
| [arismoko#2741](https://github.com/arismoko/horizon/issues/2741) | The `vehicle_health` layer can be redrawn (`gui_layer_pre`) but its heart count, which places the rows above, can't be changed | **design merged** (2026-11-10, [#2869](https://github.com/arismoko/horizon/pull/2869): mods supply the effective maximum hearts, clamped, the engine derives the rows); implementation claimed, not merged | Injection replacing `draw_mount_hud` for the vulture (`mods/vulture/native/mount_hud.rs`) | Redraw the layer from `gui_layer_pre` and supply the hearts |

## Limits of our workarounds

Each workaround works today, but each has a cost, and none is meant to stay. They all keep the mods **pinned** to one
Horizon commit (they lean on internal fields and on how a given commit behaves), and every one is code we maintain in
place of Horizon's own. Replace each one with the proper way once Horizon offers it in a release players get.

| Workaround | Its limits | Why replace it |
|---|---|---|
| Hitbox height 0.6 for the seat ([#13975](https://github.com/horizon-rust/horizon/issues/13975)) | The box is lower than the bird: its head can't be hit or right-clicked, and it fits under one-block gaps it shouldn't; anything that reads the box (collisions, arrows, the talons' reach, other mods) sees a smaller mob | The seat in `entity_type` JSON gives back a 1-block box and a seat where the harness is |
| Cloned `MobType` for the size ([#6276](https://github.com/horizon-rust/horizon/issues/6276)) | Replaces Horizon's shared type with a copy per vulture: a field Horizon adds to `MobType`, or code comparing types, may break it; it relies on `register_goals` running before the box is used | A size declared in the entity's JSON is read by every system the same way |
| Harness kept out of `Mob.extra.saddled` ([#13973](https://github.com/horizon-rust/horizon/issues/13973)) | Vanilla and other mods can't see that the vulture is saddled (a saddle drop, a mod showing mount state); forcing `saddled` to false every tick could fight a future Horizon feature | Once the steering rule only applies to horses and camels, the harness could be a real saddle state; low priority |
| Screen opened on the next tick ([#13974](https://github.com/horizon-rust/horizon/issues/13974)) | One tick of delay; the request is kept in a static keyed by entity id, single player only; if the vulture stops ticking (unloaded) the request waits or is lost | Opening it from the click is simpler and safe in multiplayer |
| Clicks in the trait hook, mounting with `Host::mount` ([#13972](https://github.com/horizon-rust/horizon/issues/13972)) | Not a workaround so much as the right path; its limit is that state changes go through `state::request`, applied on the vulture's next tick | Nothing to replace; keep it in mind for any new interaction |
| Own sounds of a muted mob ([arismoko#2738](https://github.com/arismoko/horizon/issues/2738)) | Goals have no host: sounds are queued and played on the next `entity_tick_post`; `Mob.silent` also silences what Horizon would play for us (hurt, death), so we play those too | Declared sounds play from the right place at the right time, as vanilla's |
| Sneak through `rider.sprint` ([arismoko#2739](https://github.com/arismoko/horizon/issues/2739)) | An injection into an internal function, tied to its body; Ctrl in flight can't mean anything else | A real sneak field and a cancellable dismount are the stable path |
| Render state in statics ([arismoko#2740](https://github.com/arismoko/horizon/issues/2740)) | Single player only: on a server, the client never gets the state (harness, chests, flapping); entries for unloaded vultures linger | Synced attachments read from the renderer work in multiplayer |
| Mount hearts injection ([arismoko#2741](https://github.com/arismoko/horizon/issues/2741)) | Replaces a whole internal function for one mob | A layer hook plus a row count needs no injection |
| `cargo creature install` reads `build.log` ([#13971](https://github.com/horizon-rust/horizon/issues/13971)) | Depends on the launcher's log wording ("was left out") and its path; a wording change silences it | Once the build fails by itself, keep it only as a safety net |

The same goes for the rest of our native code that reads internals (`Mob.rider`, `Mob.extra`, `MobWorld`) and for the two
injections (`mount_hud.rs`, `ride_keys.rs`): they work, but they tie us to a commit and break silently when Horizon
changes them. Moving them to stable hook points (or to the new mod system's API) is what lets a mod build on any later
Horizon without a re-release (below, "Horizon versions and our releases").

## Closed

| Issue | What | Outcome |
|---|---|---|
| [#13976](https://github.com/horizon-rust/horizon/issues/13976) | "Mod entity types from wandering_trader can't be babies or age" | Not a bug, our mistake (2026-11-09): the trader is in both lists. Our own age counter was removed (it aged vultures twice). |

## To open (candidates)

Found while developing, not filed yet. File one only with a clean repro.

- **`Host::take_items` takes nothing from a non-plain stack** (an item from the creative tab may carry component data).
  Not filed: the fork's new API no longer has `take_items` (checked 2026-11-10 on `51d735c`); see how its inventory
  services take items when we migrate. We take items only when the player isn't in creative.
- **Carrying a creature in the talons** moves it every tick with `Host::teleport_mob` (`talons.rs`). Not filed: once
  seats are in a release ([arismoko/horizon#2539](https://github.com/arismoko/horizon/pull/2539), several
  `passenger_attachments`), try mounting the caught mob with `Host::mount` on a seat below the vulture.
- **Mod keys only appear on the mod's page in Options > Mods**, not in Controls, where players look for them.
- **Suspected, unconfirmed:** an entity's attachment not readable on its first tick after loading (our "untamed one
  load in four"; we now read again next tick). Needs a repro before filing.
- **Suspected, unconfirmed:** `/summon` with `horizon:attachments` NBT doesn't set the attachments (maybe our format).

## Horizon versions and our releases

How a native mod follows Horizon (Horizon's `docs/native-mods.md`, "Pinned to a Nightly"):

- **Our mods are pinned** (`[compatibility] horizon_revision`): they read internal fields (`Mob.rider`, `Mob.extra`) and
  inject into Horizon functions, so they build on one exact commit. Every Nightly keeps its source for good
  (`nightly-source/<date>-<short>`), so **a pinned mod keeps working**: the player's launcher builds the game on our
  commit, whatever newer Horizon is out. No release is needed for each new Horizon version.
- **The cost of staying pinned:** a player with our mods plays our pinned Horizon, without the fixes released after
  it; and with another mod pinned to another commit, the launcher asks to **Build anyway** (the newest commit, where our
  mod may not apply). So we move the pin from time to time, not for every Nightly: when a release fixes one of our
  issues, when it brings an API we need, or when a popular mod has moved on.
- **What would end the pinning:** a mod that only uses Horizon's stable hook points names the API version it needs
  (`"hooks": N`, `NATIVE_HOOKS_API`) instead of a commit, and builds on any later Horizon with no re-release (Horizon's
  own Zoom, Minimap... work this way). Each workaround and internal field we drop gets us closer; the new mod system
  (below) is built for this.

Moving the pin (all modules at once, docs/ARCHITECTURE.md §6):

1. Pick the newest Nightly or release the launcher offers (`horizon-launcher versions`), take its commit.
2. Check which of our issues it fixes: a fix merged on arismoko/horizon reaches players only once it is in a
   horizon-rust/horizon Nightly. `gh api repos/horizon-rust/horizon/compare/<old>...<new> --jq '.commits[].commit.message'`
   and look for the issue numbers or the fork's PR titles; update the tables above.
3. Set the commit in every `horizon-mixin.toml`, `api/Cargo.toml` and CI's `HORIZON_REV`; `cargo creature check`;
   `cargo creature install`; drop the workarounds the new commit fixes; test in game.
4. Release: bump the modules' versions and publish a GitHub release with the `.hmod` and `.hnmod` files (Horizon's
   `docs/mods.md`, "Publish your mod").

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
