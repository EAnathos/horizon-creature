# Wyverns — specification (v0.2, draft)

A mod of the **Horizon Creature** series for **Minecraft Horizon** (Minecraft Java 26.3): a new **dark, mysterious,
mystical and mountainous dimension** ✅, reached with a staff ✅, peopled by **harpies** and **wyverns** ✅ and ruled by
the **Harpy Queen** from her crater ✅. Wyverns hatch from eggs in the library's incubator, grow up and are ridden ✅.

**Requires** the series' library, Horizon Creature (`creature`, [common/SPECS.md](../../common/SPECS.md)), **and
Butterflies** ✅ (`butterflies`, [its spec](../butterflies/SPECS.md)): the staff is crafted with butterfly dust.

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Progress (as of 2026-11-08)

- Folder and spec created. **Not started**.
- Next: the technical prototype (§1.2), the custom dimension first.

## 1. Technical context

- **Mod id**: `wyverns`. Entities `wyverns:wyvern`, `wyverns:fire_wyvern`, `wyverns:ice_wyvern`,
  `wyverns:poison_wyvern`, `wyverns:harpy_archer` ❓, `wyverns:harpy_warrior` ❓, `wyverns:harpy_queen`,
  `wyverns:tornado`, `wyverns:feather` (projectile ❓); items `wyverns:<species>_egg`, `wyverns:wyvern_saddle`,
  `wyverns:wyvern_staff` ✅; dimension `wyverns:wyvern_dimension` ✅.
- **Dependencies** ✅: `creature` and `butterflies` (`[dependencies]` in `native/horizon-mixin.toml`, `requires` in
  `module.toml`).
- Uses from the library: model drawing and sounds (§3.1, §3.2), **incubator** (§4), riding and flight (§8.1), stamina
  (§8.2), taming (§8.3), stats (§8.4), babies and growth (§8.5), ridden special attack (§8.6), ranged attacks (§8.7),
  hitbox size (§8.8). Wyverns is the mod that moves most of the vulture's systems into the library.
- Its own: the staff, the dimension, its worldgen and structures, the harpies, the Queen and her fight.

### 1.1 What the Horizon API allows (to check)

| Need | Known | Notes |
|---|---|---|
| A **custom dimension** | ❓ not documented | Only vanilla dimensions appear in Horizon's docs (`execute in`). **The main risk of the mod.** |
| Teleporting a player to it | ✅ if it exists | `execute in <dimension> run tp` works across levels |
| Hostile mod mobs | ✅ | A copy of `minecraft:zombie`, or the trader with our own target goals |
| Large hitboxes (wyverns, Queen) | ❓ | Same open point as the vulture (library §8.8) |
| Harpy riding a wyvern (mob on mob) | ❓ | Horizon's riding is player-on-mob for now; `start_riding` on a mob, or the harpy moved onto the wyvern every tick |
| Projectiles fired by a mod mob | ✅ probably | `native_summon` of a projectile with its motion; impacts through `native_projectile_impact` |
| Explosion that breaks blocks (Queen's dive) | ✅ | `native_explode(dimension, center, radius, fire, blast)` |
| Lifting players (tornadoes) | ✅ | `native_motion.rs`: a push on a client |
| Boss health bar | ❓ | Not found in the docs |
| Thrown tridents | ❌ for now | A copy of the trident doesn't throw (Horizon #3305): harpy warriors fight in melee |

### 1.2 Technical prototype (before anything else)

1. A custom dimension (simple mountainous noise) and the staff's teleport into it and back.
2. A large mob with its own hitbox.
3. A harpy riding a wyvern, both moving together.
4. A boss bar for the Queen.

If 1 is impossible today: **wait** for Horizon ✅, and look at it again later. The rest of the mod (wyverns, harpies,
eggs, the Queen) is built and tested in the Overworld meanwhile ✅; the staff and the dimension come last.

## 2. The staff ✅

- The **Wyvern Staff** ✅, crafted from **1 netherite ingot** ✅, **butterfly dust**, **a stick** and **an eye of
  ender** ✅. Proposal: shaped, eye on top, dust and netherite in the middle, stick below.
- **Four uses** ✅. Use: hold right-click for 3 s ❓ (purple particles gathering) → the player is teleported to the
  dimension, to a safe spot. Used in the dimension, it brings the player **back to the Overworld, where they used it** ✅.
  **The return trip uses a charge too** ✅: 2 round trips per staff. Broken when empty; **cannot be repaired** ✅.
- A portal may come later ❓.

## 3. The Wyvern Dimension ✅ dark, mysterious, mystical — one biome, mountainous

- **Name** ✅: the **Wyvern Dimension** (`wyverns:wyvern_dimension`).
- **One biome** ✅, **mountainous** ✅: jagged violet-grey peaks, deep ravines with mist pooling in them, floating rock
  shards around the summits, and the Queen's crater among the highest peaks. Perpetual dusk: a dim purple sky, a pale
  moon that never moves, slow-falling ash-like motes, distant wyvern calls.

### 3.1 Blocks ✅ (our design)

| Block | What it is |
|---|---|
| **Duskstone** | The dimension's dark violet stone; cobbled duskstone, bricks, polished, chiseled (a wyvern head), stairs, slabs, walls |
| **Ashen soil** / **Gloommoss** | The ground: grey soil, with a dark teal moss on top (spreads like grass) |
| **Wraithwood** | Twisted trees with black bark and pale violet leaves; logs, planks and the whole wood set (door, trapdoor, fence…) |
| **Stormglass** | Cold blue crystal clusters on the cliffs, the main light; mined into **stormglass shards** ✅ (a crafting material, the Storm Offering's) |
| **Hollow bloom** | A drooping violet flower that glows faintly at night |
| **Wyvern bone block** / **wyvern skull** | Giant bones half-buried in the cliffs, decorative skulls (placeable) |
| **Scorched**, **frosted** and **venom-stained duskstone** | The duskstone around each wyvern species' lair: scorched (glowing cracks, hurts like magma), frosted (slippery like ice), venom-stained (poisons whoever stands on it) |
| **Harpy nest** / **feather bundle** | Woven twigs and bones; bundles of grey feathers (no fall damage, like hay) |
| **Harpy banner** | Tattered feather banners on the roosts |

### 3.2 Structures ✅ (our design)

- **Arrival shrine**: a ring of duskstone pillars on a ledge with stormglass on top, where the Wyvern Staff first lands
  the player.
- **Harpy roosts** on the high peaks: nests, feather bundles and banners, a few chests; ranged harpies keep watch.
- **Harpy war camps** on wide ledges: trident racks, wyvern perches, melee harpies and wyvern riders.
- **Wyvern lairs** in cliff caves, one kind per species, with bone piles and the species' stained duskstone.
- **Bone spires**: a giant wyvern skeleton arching out of a mountain, loot in its skull.
- **The Queen's crater** (§6).

## 4. Harpies ✅ aggressive towards the player

Winged women-birds, hostile on sight ✅. Two families ✅:

| Kind | Attack | Proposal |
|---|---|---|
| **Ranged** harpy ✅ | Shoots **feathers** ✅ | Sharp feather darts (arrow-like, 4 damage ❓) in bursts of 3; keeps her distance, circles the player |
| **Melee** harpy ✅ | Fights with a **trident** ✅ | Dives at the player, trident thrusts (8 damage ❓); drops her trident rarely ❓ |
| **Wyvern rider** ✅ | From a wyvern's back ✅ | A ranged or melee harpy mounted on a wyvern: the wyvern flies, breathes and bites, the harpy shoots or strikes |

- Fly freely (no stamina), land on peaks and roosts. Groups of 2–4 ❓. Health 20 ❓.
- Drops ❓: harpy feathers (crafting: feather darts? fletching ❓), trident (rare).
- Killing a rider leaves her wyvern wild (§5) ❓.

## 5. Wyverns ✅

Four species ✅, inspired by the variety of Mo' Creatures' wyverns (our own designs and art) ✅:

| Species ✅ | Element | Proposal |
|---|---|---|
| Wyvern | — | Bite and tail strike; the most common |
| Fire wyvern | Fire | Fire breath (sets the target on fire), immune to fire and lava ❓ |
| Ice wyvern | Ice | Frost breath: slowness and freezing ❓, immune to freezing |
| Poison wyvern | Poison | Poison spit, tail sting (poison) |

- **Wild** wyverns are **neutral, hostile once hurt** ✅ (they fight back until the attacker is dead or far ❓); and
  **ridden** by harpies ✅ (those attack with their rider).
- Bigger than the vulture (hitbox ~2 × 2 ❓, wingspan ~5 blocks ❓). Health 50 ❓.

### 5.1 Eggs ✅

- Eggs are obtained **by killing wyverns** ✅: **15 %** ✅ per wyvern, an egg of its species.
- Incubated in the library's incubator **like a snake's egg, but hotter** ✅, at **a different temperature per species**
  ✅ (±10 °C, faster the closer, library §4.1; ~20 min ❓ at the exact temperature). Proposal ❓:

  | Egg | Temperature ❓ |
  |---|---|
  | Ice wyvern | 64 °C |
  | Wyvern | 91 °C |
  | Poison wyvern | 107 °C |
  | Fire wyvern | 134 °C |
- It hatches as a **juvenile** wyvern tamed to the player who put the egg in ✅.

### 5.2 Growing up and riding ✅

- From juvenile to **fully grown: 1 hour** ✅ (growing visibly; faster when fed ❓, with raw meat / its element's food ❓).
- It **can be mounted from 40 minutes** ✅ (still a bit smaller and slower ❓), with **its own saddle** ✅
  (`wyverns:wyvern_saddle`, its own recipe ✅; proposal ❓: 5 leather in a U, 2 harpy feathers, 1 iron ingot in the
  middle, 1 string).
- Flight with stamina like the vulture, through the library (§8.1, §8.2) ✅; numbers per species ❓.
- **The rider controls the breath** ✅: a key (the library's ridden special attack, §8.6) breathes the species' element
  where the rider looks; costs stamina ❓; the plain wyvern bites / tail-sweeps instead ❓.
- Tamed wyverns: sit / follow, defend the owner (library §8.3) ❓; keep their element and immunities ✅.

## 6. Boss: the Harpy Queen ✅

### 6.1 Home and look

- She lives in **a crater** ✅ among the mountains: a vast bowl of shattered duskstone, Stormglass shards around the rim,
  roosts on the slopes, her **Bone Throne** in the centre (an unbreakable block). The first time, entering the bowl wakes her ✅.
- **Much bigger** than a harpy (~3.5 blocks tall ❓), **muscular, powerful, intimidating**, a boss at a glance ✅:
  dark slate plumage, a pale bone crown, scarred wings (span ~8 blocks ❓), glowing violet eyes, talons of black iron.

### 6.2 Numbers

- **One Queen at a time** ✅: she lives in the crater; once killed she is gone until the player **summons her again**
  (§6.6). The crater keeps the scars of the fight (her dives break blocks); the Bone Throne can't be broken.
- Health **400** ❓ (the Wither 300, the Warden 500), armour 10 ❓, knockback resistance 0.8 ❓, immune to fall damage
  and to her own explosions and tornadoes.
- A boss bar with her name ❓ (§1.1).

### 6.3 Attacks ✅ (dynamic, several, never just plain blows)

1. **Supersonic dive** ✅: she climbs high above the crater, screeches (the warning, ~1.5 s, a trail of light locks on the
   player ❓), then **charges at very high speed**, hits the ground and **explodes like a Creeper** ✅ (power 3 ❓), **breaking
   blocks** ✅. She stays **stuck in the crater floor ~3 s** ❓ after it: the opening for melee (double damage ❓).
2. **Tornadoes** ✅: she summons 2 ❓ tornadoes that wander towards players, **lift them into the air** ✅ and **deal
   damage** ✅ (2 per second ❓ while caught, then a drop: fall damage). They last 10 s ❓.
3. **Melee** ✅ with stylised moves ✅ (each with its own animation):
   - **Talon rake**: a two-hit combo (10 + 6 ❓);
   - **Wing buffet**: a sweeping blow that knocks everyone around her away ❓;
   - **Grab and throw**: seizes a player, climbs and throws them ❓ (8 + fall damage);
   - **Spin strike**: a whirling dash through the player ❓.
4. **Feather storm** ❓: a fan of 9 steel feathers, or a rain from above the arena.
5. **Shriek** ❓: a sonic scream; slowness II and nausea for 5 s in 12 blocks.

### 6.4 Phases ✅ three

| Phase | Health | Behaviour |
|---|---|---|
| 1 — Sovereign | 100–60 % | Melee combos, feather storm, a dive every ~25 s |
| 2 — Tempest | 60–30 % | Adds tornadoes and the shriek; calls 2 ranged harpies; dive every ~18 s |
| 3 — Fury | below 30 % | Faster everything; **chained dives** (2–3 in a row); wind pulls players towards the crater's centre; the rim's roosts empty |

- An attack pattern with cooldowns, never the same attack twice in a row ❓.

### 6.5 Reward ✅

- The **Harpy Head trophy** ✅ (`wyverns:harpy_queen_head`): a placeable head of the Queen, on the floor or a wall, like
  vanilla mob heads; plus XP (500 ❓).

### 6.6 Summoning her again ✅ (our design)

The harpies covet wyvern eggs: the Queen comes back for one.

- **Storm Offering** (`wyverns:storm_offering`), crafted from **1 wyvern egg** (any species) in the middle, **4 harpy
  feathers** on the sides and **4 stormglass shards** in the corners. Unstackable.
- **Right-click the Bone Throne** with it, in the crater, while no Queen is alive: the offering is placed on the throne
  and the ritual starts.
- **The ritual, 10 s**: the sky over the crater darkens, the wind rises and pulls loose items towards the throne,
  lightning strikes the rim's stormglass, the roosts fill with harpies' cries; at the end, a tornado touches down on the
  throne, the offering is gone, and the Queen lands from it with a shriek, at full health, starting in phase 1.
- Refused (the offering stays in hand, with a message) if she is alive, or if the player is not in the crater.
- Every fight gives the reward again (§6.5): a Harpy Head trophy per kill.

## 7. Out of scope for v1

- Multiplayer (v2, the whole series).
- More species or bosses (later, without changing the architecture).

## 8. Open questions

- ~~Q2 — Access to the dimension~~ A four-use staff (netherite, butterfly dust, stick, eye of ender).
- ~~Q1 (part)~~ One mountainous biome; the Queen lives in a crater.
- ~~Q3~~ Ranged harpies shoot feathers. ~~Q4~~ Wild wyverns are neutral, hostile once hurt.
- ~~Q5 / Q6 / Q7 (part)~~ Eggs by killing wyverns, hotter than snakes; grown in 1 h, ridden from 40 min with a saddle; the
  rider controls the breath.
- ~~Q8 — No custom dimensions yet?~~ Wait, and look at it later.
- ~~Q9 — Staff?~~ Netherite ingot; the return trip uses a charge and brings you back where you left the Overworld; not
  repairable.
- ~~Q11 (part) / Q12~~ A different temperature per species; our own saddle and recipe.
- ~~Q10~~ The Wyvern Dimension and the Wyvern Staff; blocks and structures as designed (§3).
- ~~Q11 (part)~~ 15 % egg drop.
- ~~Q13 (part)~~ One Queen at a time, three phases, the Harpy Head trophy; summoned again with a Storm Offering on her
  Bone Throne (§6.6).
- Q11 — The temperatures per species (§5.1).
- Q15 — The Queen's numbers (§6.2), to tune in game.
- Q14 — Can the player tame a harpy's wyvern after killing its rider, or only hatched ones?
