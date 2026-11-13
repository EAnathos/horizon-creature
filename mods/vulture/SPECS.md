# Vulture: specification

A mod of the **Horizon Creature** series for **Minecraft Horizon** (a Rust reimplementation of Minecraft Java 26.3).
It adds a wild vulture that you tame by throwing raw chicken at it, ride, fit with chests, that catches creatures in its
talons and breeds.

**Requires** the series' library, Horizon Creature (`creature`, [common/SPECS.md](../../common/SPECS.md)). Systems the
library shares are described there; this spec gives the vulture's own rules and numbers.

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

Done and tested in game (single player). What waits for v2 is in §9.

- **The vulture** `vulture:vulture` (a copy of the wandering trader with its own goals), its own 1.2 × 1.0 box, drawn by
  the mod from `assets/models/vulture.bbmodel` (§7.1); a chick model for babies (`baby.bbmodel`). Red when hurt,
  tipping over when dying. Its own sounds from CC0 freesound recordings (§7, `assets/sounds.toml`), English and French
  subtitles.
- **Wild** (§4): spawns in deserts, badlands and savannas; soars in circles and lands on high ground; walks about and
  looks at players. Neutral: it fights back from the air (it hovers over its target's head, striking, then pulls
  away), and nearby vultures join in. Drops feathers and meat (raw, or cooked when it burns); a vulture feather crafts
  arrows, the brush and the book and quill in place of a feather.
- **Stats** (§2): each rolls its max health and flight speed once.
- **Taming** (§5): it walks to raw meat thrown on the ground and eats it; a raw chicken tames it one time in four.
- **Tamed** (§6): sits or follows, flies to its owner and lands near them, defends them. Its owner fits it with a plain
  **saddle** (drawn as a seat and a breast strap) and up to two **chests**; sneak + right-click opens its screen
  (saddle, sit / follow, the chests' slots). Leads work, wild or tamed.
- **Riding and flight** (§6.2): right-click to mount; it flies like a plane, steered by the look and A / D, Space climbs, sneak
  descends, getting off only on the ground; only the hearts it has are shown. **Talons** (§6.4): R catches a small
  creature below and carries it.
- **Breeding** (§8): rotten flesh puts two tamed vultures in love; their baby averages their stats, grows from a chick
  to a juvenile to an adult in 20 minutes, faster when fed raw meat. A chick doesn't follow its owner, a juvenile does.
- **Bestiary page**: the biomes a wild one was met in, "Feed" (raw chicken) and "Breeding" (rotten flesh).

Left: tuning (flight numbers, taming odds, speeds, the dive attack) as players report.

---

## 1. Technical context

- **Format**: a Horizon native mod = a content mod (items, recipes, loot, textures, sounds, entity type in JSON)
  + Rust code built into Horizon (`horizon-mixin.toml`, `native_*` hooks). Tool: `cargo horizon-mod`.
- **Toolchain**: the Rust the Horizon launcher pins.
- **Target**: game 26.3, on the series' pinned Horizon revision (docs/HORIZON-ISSUES.md).
- **Mod id**: `vulture` (ids kept from before the series, so test worlds still load).
- **Dependency** ✅: the library `creature` (`[dependencies] creature = "^0.1"` in `native/horizon-mixin.toml`). Entity `vulture:vulture`, items `vulture:feather`, `vulture:raw_vulture`,
  `vulture:cooked_vulture` (the saddle is vanilla's).
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
| Mod sounds | ✅ | `assets/vulture/sounds.json` + `.ogg`, played by `Host::play_sound` (Horizon always prefixes mob sounds with `minecraft:`) |
| Inventory screen | ✅ | Mod menu types (1 to 256 slots) |
| Natural spawning | ✅ | `add_spawns` by biome |
| **Riding a mod entity and flying it** | ✅ | `/ride` accepts any mob; Horizon copies the rider's keys into `Mob.rider` (forward, strafe, jump, sprint), which a goal reads to set the speed. Internal fields: Horizon revision pinned. Right-click → mount: `Host::mount` from the server's `interact_entity` hook. No cheats needed. |
| Carrying an entity (talons) | ✅ | Teleported under the vulture every tick (`talons.rs`). |

---

## 2. Variable stats ✅ (like horses)

> The vulture's own system for now (a candidate for the library: common/SPECS.md §6). Its ranges and rules below.

Each vulture rolls its stats when born or spawned:

| Stat | Range | Effect |
|---|---|---|
| Max health | 16 – 30 | |
| Flight speed | 0.6 – 1.2 × base speed | Cruising and diving speed (§6.2) |

- **Breeding**: as with vanilla horses, each stat of the baby is the average of both parents and a random value. Good
  bloodlines improve over time without passing the caps.
- Stats are never shown in game ✅, as in vanilla.

## 3. Skin variants (v2) ✅

**Not in v1** ✅: v1 has one look, the tawny vulture. The variants come in v2 (§9). Inspired by real species, each with
its own texture:

| Variant | Inspiration | Spawning |
|---|---|---|
| Tawny (`tawny`, FR fauve) | Griffon vulture (beige, white ruff) | Common, all biomes |
| Dark (`dark`, FR sombre) | Cinereous vulture (very dark brown) | Common, badlands |
| White (`white`, FR blanc) | Egyptian vulture (white, yellow face) | Common, desert |
| Bearded (`bearded`, FR barbu) | Bearded vulture (orange breast, black "beard") | Uncommon, savanna plateau / nearby mountains |
| King (`king`, FR royal) | King vulture (multicoloured head) | Rare (~3 %), everywhere |

Variant names are simple colour names ✅, so players need not know the species.

- A baby takes the variant of one of its parents (50/50), with a ~5 % chance of mutating to a random variant ✅.
- Secondary markings (spots, darker wings) could multiply the combinations, as for horses: later still ✅.

## 4. Wild vulture

| Property | Value |
|---|---|
| Size (hitbox) | ~1.2 × 1.0 blocks; visual wingspan ~4.5 blocks in flight (§7.1). Horizon gives a mod mob its template's size (the trader's 0.6 × 1.95): we try overriding it at spawn, else we keep the trader's ✅ |
| Behaviour ✅ | **Neutral and retaliates**: attacks whoever hits it (3 damage), and nearby vultures join in, like wolves/llamas |
| Biomes ✅ | Desert, badlands, savanna; groups of 1–3 |
| Wild flight | Slow circles high up, lands on high points |
| Scavenger | Drawn to meat on the ground and recently dead mobs (~16 blocks) |
| Drops ✅ | 0–2 **vulture feathers** (a feather in arrows, the brush, the book and quill ✅) + 1–2 **raw vulture meat** (cookable, ordinary meat ✅) |

## 5. Taming ✅

> Ownership, sit/follow and defending the owner are candidates for the library (common/SPECS.md §6.2); taming by thrown chicken is the
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
  - right-click = mount if it is saddled and you are its owner (only the owner rides it ✅), otherwise sit/follow;
  - sneak + right-click = the vulture's screen (which also has a sit/follow button);
  - right-click with rotten flesh = breeding mode (§8).

### 6.1 Saddle ✅

- A plain **saddle** (vanilla's `minecraft:saddle`) ✅, drawn as a seat and a breast strap (the code and the saves call
  it the harness).
- Needed to ride it **and** to carry chests ✅. Goes in a slot of the vulture's screen, and shows on the model.

### 6.2 Dynamic flight ✅

> Riding and flight stay in the vulture until a second mod needs them (common/SPECS.md §6.1). The vulture's keys and numbers below. **No
> stamina** ✅: the vulture flies as long as the rider wants.

Flown like a plane, not like a flying camera: the vulture always moves forward along its heading, the rider turns it
by **looking** where it should go ✅ and sets its height with **Space and sneak** ✅.

| Rider | Vulture |
|---|---|
| Looks **right / left** | **Banks** to that side and **turns in an arc** ✅, like a plane: the further the look is from its heading, the steeper the bank (up to ~50°) and the tighter the turn (up to ~5° a tick); it levels out as its heading reaches the look |
| A / D (Q / D on AZERTY) | Steers to that side ✅: its wanted heading is 45° from the look while held, so it banks and turns the same way |
| W (Z on AZERTY) | Flaps: speeds up to its cruising speed (wing beats, flap sound) |
| (nothing) | Glides: keeps its speed slowly falling off, sinks gently when level |
| S | Air brake ✅: slows down, much faster than a glide (and so sinks more, down to a stall) |
| Space (held) | Noses up and **climbs** with strong flaps ✅ (loses speed); on the ground, **takes off** |
| Sneak (held) | Noses down and **descends** ✅ (gains speed, up to the dive speed); Ctrl does the same |
| (neither) | Flies level |
| Sneak on the ground | Get off ✅: **only on the ground**, never in the air |
| "Talons" key ✅ (R, rebindable in Controls) | Catch / release (§6.4) |

- **Speed and pitch** trade like a glider's ✅: diving builds speed, climbing spends it. Too slow while climbing, it
  stalls and noses down by itself ✅, so it can't hang in the air.
- The **body follows the flight**: pitched with the climb or dive, rolled with the bank, the wings spread wide in a
  glide, beating when it flaps, swept back in a steep dive ✅.
- The pitch and the turn rate follow the look smoothly (no snapping), so the flight reads as momentum.
- The look's pitch does nothing (the camera is free): height is Space and sneak.
- On the ground: a slow walk (W, A, D), **S backs it up** at less than half that pace ✅ (like a big flier stepping
  back), Space takes off, sneak gets off.
- Horizon gets a rider off on sneak before any mod hears of it: the `ride-keys` injection (`ride_keys.rs`) keeps them
  on in the air.
- ✅ Chests **do not weigh** on its flight. ✅ **No thermals.**
- Starting numbers ✅, to balance in game (blocks per tick): cruise 0.55 × its speed stat (× 2 ridden), dive up to 1.0 (× 2 ridden), slowest 0.18 before a stall,
  glide sink 0.04 when level, take-off climb 0.4, descent at least 0.2.

### 6.3 Chests ✅

- Up to **2 chests**, one on each side, visible on the model. Added by right-clicking with a chest (saddle required).
- **15 slots per chest**, so 30 in all.
- **Vulture screen**: saddle slot, "remove left / right chest" buttons ✅ (the contents drop on the ground), a grid of 15
  or 30 slots, sit/follow button.
- On death it drops its chests, their contents and its saddle.

### 6.4 Talons ✅

- In flight, ridden, with the talons key: catches the nearest creature below (up to 4 blocks down, 2.5 around ✅,
  1.5 was too tight). The legs thrust down on every try, caught or missed ✅.
- It can catch:
  - **small enough mobs** (hitbox ≤ 1 block wide and ≤ 1 block high ✅: chicken, rabbit, pig, sheep, fox, cat, wolf,
    babies…);
  - **allies** ✅: a player sneaking under the vulture (sneaking means consent), or a tamed animal.
- What it carries **does not weigh** on its flight ✅, like the chests (§6.3).
- The key is a mod key, R by default, changed on the vulture's page in Options > Mods; a sneaking player under it can't be
  caught in single player (the only player is its rider), so allies are tamed animals for now.
- It lets go on the key, when the vulture dies or when the rider gets off. The fall deals normal damage.
- A wild or unridden vulture catches nothing ✅.

## 7. Graphics and audio

- The **model is drawn by code** (`native_mob_submit`) from cubes. Its source is a `.bbmodel` file (Blockbench),
  converted to Rust data by a script in the repository: we edit visually, the game gets cubes, pivots and UVs.
- Animated parts: head, neck, body, 2 wings in three segments (arm, forearm, hand with its primaries), tail, 2 legs with
  talons, saddle, 2 chests.
- Code animations: flap, glide (wings spread, still), banking turn, dive (wings swept back), walk, peck/eat, sit,
  attack, closed talons.
- Sounds: call, wing flap, eat, hurt, death. CC0 sounds (freesound.org, licence checked; CC0 requires no credit, the
  sources are listed in `tools/src/sounds.rs`), edited.
- English and French translations.

### 7.1 Model ✅ (redesign)

**More worked and imposing** ✅, after photos of griffon and Rüppell's vultures (landing with wings spread, perched,
wings open on a post), and **lean, not stocky** ✅ (a first version looked like a turkey): the profile of a hunched
vulture.

- **Silhouette** ✅: a long, narrow, low body (8 × 7 × 16 texels) whose back **slopes down to the tail** on the ground,
  a cream **ruff** at the shoulders, the neck **reaching forward** from it at shoulder height (not upright), a small bald
  head held low with a **heavy hooked beak**, short legs with feathered thighs, grey feet and dark talons, a short square
  tail. No breast bulge.
- **Wings spread** ✅, the impressive part: long and broad (span **~4.5 blocks**), three segments (arm, forearm, hand)
  so they fold along the flanks on the ground and open fully in flight. The hand ends in **separate primary feathers,
  the "fingers"** ✅ (5 per wing). Two-tone like the photos: scaly, mottled brown-and-cream **coverts** on top, then a
  pale band, then **dark brown flight feathers**.
- **Folded** on the ground, the wings cover the flanks, coverts on top, and their primaries reach past the tail.
- **Colours** (tawny, v1): mottled brown and cream body, cream ruff, grey-pink bare head and neck, a horn-coloured beak
  with a dark tip, dark eyes.
- Texture **128 × 128** ✅ for the detail.
- **In flight** the neck stays forward, the legs are tucked.

### 7.2 Spawn egg ✅ (redesign)

- In the style of the **latest vanilla spawn eggs** ✅ (since 1.21.5, each egg is drawn for its mob instead of two
  colours with spots): an egg shape with a dark outline and shading, painted with the mob's look.
- The vulture's: a mottled brown egg with dark flight-feather streaks on its sides, a cream ruff collar around its top
  third, and the bald grey-pink head with its hooked beak peeking from the top ✅. 16 × 16, readable at a glance next to
  vanilla eggs.

## 8. Breeding ✅ (v1)

> A candidate for the library: common/SPECS.md §6.3. The vulture gives birth to a live baby (no egg) ✅.

- Two tamed vultures fed **rotten flesh** ✅ (scavengers…) go into breeding mode.
- The baby is born tamed, with the same owner, and inherits stats (§2) (and its variant once there are variants, §3).
- The baby is smaller, with a big head and a downy crown. It grows up in 20 min (faster when fed, with hearts), and can
  only be ridden once grown. It is seen growing: a chick for the first half, then a juvenile (the adult's shape, smaller)
  growing to full size ✅.
- 5 min cooldown before the parents can breed again.

## 9. v2 (later, kept out of the v1 to-do list) ✅

- Multiplayer: LAN and dedicated server. For a LAN guest it is their own client that moves the mount.
- The skin variants (§3): dark, white, bearded and king, their textures and their spawning by biome; then secondary
  markings.
- Vulture armour.
- User configuration.

## 10. Remaining open questions

- ~~Q27: The flight's numbers, the bank and turn limits, and what A / D and S do in the air?~~ The numbers as built, to
  balance in game; A / D steer, S brakes in the air and backs up on the ground (§6.2).
- ~~Q28: What a passenger in its talons does to its flight?~~ Nothing: no weight (§6.4).
