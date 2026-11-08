# Vulture: specification (v0.2)

A mod of the **Horizon Creature** series for **Minecraft Horizon** (a Rust reimplementation of Minecraft Java 26.3).
It adds a wild vulture that you tame by throwing raw chicken at it, ride, fit with chests, that catches creatures in its
talons and breeds.

**Requires** the series' library, Horizon Creature (`creature`, [common/SPECS.md](../../common/SPECS.md)). Systems the
library shares are described there; this spec gives the vulture's own rules and numbers.

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Progress (as of 2026-11-08)

### Done

- **Phase 0 succeeded** (§1.2): entity `vulture:vulture` (a copy of the wandering trader with its goals replaced), drawn
  by the mod, ridden and flown. Tested in game, single player.
- **Mounting**: right-click on the vulture (no cheats needed) or `/ride` (cheats). Holding sneak gets off (confirmed in
  game).
- **Ridden flight**, without stamina for now: WASD (ZQSD on AZERTY) following the look direction, Space climbs, nothing =
  glide, Ctrl = dive. The vulture takes no fall damage.
- **Model and texture of the tawny vulture**: `mods/vulture/assets/models/vulture.bbmodel` (Blockbench, 17 cubes: head, hooked
  beak, neck, ruff, body, 2-segment wings, tail, legs and talons), 64×64 texture, converted to Rust by
  `cargo creature model vulture` and drawn by the library (`creature::model`). Animations: wings folded on the ground, flapping in flight, head following the gaze, walk
  cycle, legs tucked in flight.
- **Mount hearts**: only the hearts it has, no empty ones.
- **Sounds** (§7): the trader is muted; call (hiss, 3 variants + a rare call), hurt, death, wing flaps, takeoff and eat
  (those two not played by the code yet), made from CC0 freesound sounds by `cargo creature sounds vulture` (sources
  listed in `tools/src/sounds.rs`), played by the library (`creature::sounds`). English and French subtitles. Hurt and death: written, not tested in game yet.
- **Spawn egg** `vulture:vulture_spawn_egg`, in the series' "Horizon Creature" creative tab (2026-11-08; the vulture's
  own tab is gone).
- **Moved into the Horizon Creature series** (2026-11-08): `mods/vulture/`, depends on the library; built with the
  series' tools (`cargo creature install vulture`), tested in game.

### To do

- §2 Variable stats (health, speed, stamina) and their inheritance: in the library (common/SPECS.md §8.4) if Wyverns
  is under way by then, else here first.
- §7 Model: longer neck and smaller head, closer to a real vulture's silhouette, before the other variants.
- §3 The other 4 variants (dark, white, bearded, king): textures and rolling one at spawn.
- §4 Wild vulture: hitbox (~1.2 × 1.0; it still has the trader's 0.6 × 1.95: try overriding its mob type's size at
  spawn), AI (circling in flight, landing, scavenging), neutral
  and retaliating, natural spawning by biome, drops (feathers, meat) and their items.
- §5 Taming with thrown raw chicken, owner.
- §6 Tamed vulture: sit/follow, defending its owner, healing; riding limited to the owner, with a harness.
- §6.1 Harness (item, recipe, slot, visible on the model).
- §6.2 Stamina: bar in place of the XP bar, costs and recovery; walking on the ground and taking off.
- §6.3 Chests (2 × 15 slots), the vulture's screen.
- §6.4 Talons: key, catch and release.
- §7 Baby (model and texture), harness and chests on the model, peck/eat, sit, attack and talon animations.
- §8 Breeding.
- v2: multiplayer (LAN and dedicated server). For a LAN guest it is their own client that moves the mount.

---

## 1. Technical context

- **Format**: a Horizon native mod = a content mod (items, recipes, loot, textures, sounds, entity type in JSON)
  + Rust code built into Horizon (`horizon-mixin.toml`, `native_*` hooks). Tool: `cargo horizon-mod`.
- **Toolchain**: Rust 1.99.1, pinned by the launcher and already installed.
- **Target**: game 26.3, Horizon revision pinned at the start of development.
- **Mod id**: `vulture` (ids kept from before the series, so test worlds still load).
- **Dependency** ✅: the library `creature` (`[dependencies] creature = "^0.1"` in `native/horizon-mixin.toml`). Entity `vulture:vulture`, items `vulture:harness`, `vulture:feather`, `vulture:raw_vulture`,
  `vulture:cooked_vulture`.
- **Multiplayer** ✅: v1 = single player. LAN and the dedicated server come in v2 (for a LAN guest, their own client
  moves the mount, which needs its own work), but server logic is kept apart from rendering from the start.
- **Languages** ✅: everything in the repository is in English; the game text is translated (`en_us`, `fr_fr`).

### 1.1 What the Horizon API allows today (read in Horizon's `docs/mods.md` and `docs/native-mods.md`)

| Need | Available | Notes |
|---|---|---|
| New living entity type | ✅ tested in game | Only by copying a **zombie, villager or wandering trader**. `vulture:vulture` (wandering trader) works: `/summon`, egg, saving. |
| Custom AI | ✅ | `register_goals`: we clear the base model's (wandering trader's) goals and add ours |
| Custom model and texture | ✅ tested in game | `native_mob_submit`: we draw our cubes (`native_draw_cube`, pose stack, pivots) with `vulture:entity/...` textures |
| Per-entity saved data | ✅ | Server attachments on `Holder::Entity(id)` (owner, stats, variant, chests…) |
| Right-click on the entity | ✅ | `native_interact_entity` |
| Taming event | ✅ | `native_entity_tame` (vanilla only, to check) |
| Nearby items on the ground | ✅ | API 42 (position, stack, pickup delay) |
| Mod sounds | ✅ event played in game (`.ogg` files not listened to yet) | `assets/vulture/sounds.json` + `.ogg`, played by `Host::play_sound` (Horizon always prefixes mob sounds with `minecraft:`) |
| Inventory screen | ✅ | Mod menu types (1 to 256 slots) |
| Natural spawning | ✅ | `add_spawns` by biome |
| **Riding a mod entity and flying it** | ✅ phase 0 (2026-11-07) | `/ride` accepts any mob; Horizon copies the rider's keys into `Mob.rider` (forward, strafe, jump, sprint), which a goal reads to set the speed. Internal fields: Horizon revision pinned. Right-click → mount: a mixin injection that calls `start_riding` (`run_command` is refused during a player's turn). No cheats needed. |
| Carrying an entity (talons) | ❓ | Probably by making a creature another's passenger, or by teleporting it every tick. To check. |

### 1.2 Phase 0: technical prototype, before anything else

1. Declare `vulture:vulture` (a copy of the wandering trader), spawn it with `/summon` and draw it with a cube.
2. Mount a player on it and read their keys (WASD, Space, Ctrl) on the server.
3. Make it fly by setting its speed directly.

If step 2 or 3 is not possible, three options:
- **A**: contribute to Horizon (open source) a mount-type base model (horse, donkey, happy ghast);
- **B**: spawn a vanilla donkey, which can already be ridden, carry chests, vary its stats and breed, and draw it as a
  vulture;
- **C**: write the mount ourselves with player input and position hooks.

---

## 2. Variable stats ✅ (like horses)

> Shared system: common/SPECS.md §8.4. The vulture's ranges and rules below.

Each vulture rolls its stats when born or spawned:

| Stat | Range | Effect |
|---|---|---|
| Max health | 16 – 30 | |
| Flight speed | 0.6 – 1.2 × base speed | Glide and dive |
| Max stamina | 70 – 130 | Flapping flight time (§6.2) |

- **Breeding**: as with vanilla horses, each stat of the baby is the average of both parents and a random value. Good
  bloodlines improve over time without passing the caps.
- Stats are not shown in game, as in vanilla ✅.

## 3. Skin variants ✅

Inspired by real species, each with its own texture:

| Variant | Inspiration | Spawning |
|---|---|---|
| Tawny (`tawny`, FR fauve) | Griffon vulture (beige, white ruff) | Common, all biomes |
| Dark (`dark`, FR sombre) | Cinereous vulture (very dark brown) | Common, badlands |
| White (`white`, FR blanc) | Egyptian vulture (white, yellow face) | Common, desert |
| Bearded (`bearded`, FR barbu) | Bearded vulture (orange breast, black "beard") | Uncommon, savanna plateau / nearby mountains |
| King (`king`, FR royal) | King vulture (multicoloured head) | Rare (~3 %), everywhere |

Variant names are simple colour names ✅, so players need not know the species.

- A baby takes the variant of one of its parents (50/50), with a ~5 % chance of mutating to a random variant ✅.
- Secondary markings (spots, darker wings) could multiply the combinations, as for horses: **later** ✅ (v1 = the 5
  variants).

## 4. Wild vulture

| Property | Value |
|---|---|
| Size (hitbox) | ~1.2 × 1.0 blocks; visual wingspan ~2.5 blocks in flight. Horizon gives a mod mob its template's size (the trader's 0.6 × 1.95): we try overriding it at spawn, else we keep the trader's ✅ |
| Behaviour ✅ | **Neutral and retaliates**: attacks whoever hits it (3 damage), and nearby vultures join in, like wolves/llamas |
| Biomes ✅ | Desert, badlands, savanna; groups of 1–3 |
| Wild flight | Slow circles high up, lands on high points |
| Scavenger | Drawn to meat on the ground and recently dead mobs (~16 blocks) |
| Drops ✅ | 0–2 **vulture feathers** + 1–2 **raw vulture meat** (cookable, ordinary meat ✅) |

## 5. Taming ✅

> Ownership, sit/follow and defending the owner are shared (common/SPECS.md §8.3); taming by thrown chicken is the
> vulture's own.

1. The player **throws** a raw chicken (Q key) near a wild vulture. Handing it over does not work.
2. The vulture spots the item (~16 blocks), lands on it and eats it (animation + particles).
3. **1 chance in 4** per chicken: hearts on success, smoke otherwise.
4. The owner is the player who threw the chicken.
5. Other raw meats are eaten with no effect; cooked chicken is ignored.

## 6. Tamed vulture ✅ (like the wolf)

- **Sit / follow**: when following, it flies above its owner and teleports if too far away.
- Defends its owner and attacks what the owner hits, like the wolf.
- Can be named with a name tag. No fall damage. Healing: raw meat (+4 HP).
- **Controls** ✅:
  - right-click = mount if it has a harness and you are its owner (only the owner rides it ✅), otherwise sit/follow;
  - sneak + right-click = the vulture's screen (which also has a sit/follow button);
  - right-click with rotten flesh = breeding mode (§8).

### 6.1 Harness ✅

- **Vulture harness** item, unstackable, recipe: 5 leather + 2 string + 1 iron ingot (U shape, iron in the middle) ✅.
- Needed to ride it **and** to carry chests ✅. Goes in a slot of the vulture's screen, and shows on the model.

### 6.2 Flight with stamina ✅

> Riding, flight and stamina are shared with Wyverns (common/SPECS.md §8.1, §8.2). The vulture's keys and numbers below.

| Key | Action |
|---|---|
| WASD (ZQSD) | Direction following the look |
| Space | Flap: gains height, uses stamina |
| (nothing) | Glide: slow forward descent |
| Left Ctrl | Dive: fast descent, gains speed |
| Sneak | Get off, **even in mid-air** ✅ (normal fall damage) |
| "Talons" key ✅ (R, rebindable in Controls) | Catch / release (§6.4) |

- Stamina bar in place of the XP bar while riding, like the horse's jump bar.
- Flapping: −20/s · Gliding: +5/s · On the ground: +25/s.
- At 0 it cannot flap again until back at 30 %: it glides down.
- ✅ Chests **do not weigh** on stamina. ✅ **No thermals.**
- On the ground: slow walk, takes off with Space.
- Starting speeds ✅ (blocks per tick, before air drag): flight 0.6, dive 1.0, climb 0.35, glide sink 0.08, dive sink
  0.7 (felt right in game).

### 6.3 Chests ✅

- Up to **2 chests**, one on each side, visible on the model. Added by right-clicking with a chest (harness required).
- **15 slots per chest**, so 30 in all.
- **Vulture screen**: harness slot, "remove left / right chest" buttons ✅ (the contents drop on the ground), a grid of 15
  or 30 slots, sit/follow button.
- On death it drops its chests, their contents and its harness.

### 6.4 Talons ✅

- In flight, ridden, with the talons key: catches the creature just below (~3 blocks).
- It can catch:
  - **small enough mobs** (hitbox ≤ 1 block wide and ≤ 1 block high ✅: chicken, rabbit, pig, sheep, fox, cat, wolf,
    babies…);
  - **allies** ✅: a player sneaking under the vulture (sneaking means consent), or a tamed animal.
- Carrying someone **costs stamina** ✅: flaps cost +50 % ✅ and gliding no longer recovers.
- It lets go on the key, when the vulture dies or when the rider gets off. The fall deals normal damage.
- A wild or unridden vulture catches nothing ✅.

## 7. Graphics and audio

- The **model is drawn by code** (`native_mob_submit`) from cubes. Its source is a `.bbmodel` file (Blockbench),
  converted to Rust data by a script in the repository: we edit visually, the game gets cubes, pivots and UVs.
- Animated parts: head, neck, body, 2 two-segment wings (arm + tip), tail, 2 legs with talons, harness, 2 chests.
- Code animations: flap, glide, dive, walk, peck/eat, sit, attack, closed talons.
- 64×64 textures: one per variant + baby + harness, painted in Blockbench.
- Sounds: call, wing flap, eat, hurt, death. CC0 sounds (freesound.org, licence checked; CC0 requires no credit, the
  sources are listed in `tools/src/sounds.rs`), edited.
- Spawn egg, item textures, English and French translations.

## 8. Breeding ✅ (v1)

> Shared system: common/SPECS.md §8.5. The vulture gives birth to a live baby (no egg, no incubator) ✅.

- Two tamed vultures fed **rotten flesh** ✅ (scavengers…) go into breeding mode.
- The baby is born tamed, with the same owner, and inherits stats (§2) and a variant (§3).
- The baby is smaller, with a big head and a downy crown. It grows up in 20 min (faster when fed), and can only be ridden
  once grown.
- 5 min cooldown before the parents can breed again.

## 9. Out of scope for v1

- Multiplayer: LAN and dedicated server (v2)
- Vulture armour
- Showing stats in game
- User configuration

## 10. Remaining open questions

- ~~Q26: If phase 0 shows a mod entity cannot be ridden: A, B or C (§1.2)?~~ Moot: phase 0 succeeded without any of the
  three options (2026-11-07).
- ~~Extra stamina cost when carrying someone: +50 %?~~ Decided: +50 % (§6.4).
