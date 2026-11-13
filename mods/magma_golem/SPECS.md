# Magma Golem: specification

A mod of the **Horizon Creature** series for **Minecraft Horizon** (Minecraft Java 26.3): a giant of living magma,
six blocks tall, that sleeps in the Nether as a heap of glowing rock until someone comes too close. A rare mini-boss:
found, not summoned, fought with a boss bar, and cooled with snow.

**Requires** the series' library, Horizon Creature (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

Planned: the spec and the model, no code yet.

- **The model** (`assets/models/magma_golem.bbmodel`, pictures in docs/MODELS.md): our own design, about 6 blocks
  tall, built from boulders of dark grey-brown rock split by glowing orange seams, lava showing at the belly, the
  chest, the elbows, the wrists and the knees. Hunched forward like an ape: a massive back and huge shoulders rise
  above its head, held low in front: plain dark stone, a heavy overhanging brow, two glowing eyes in dark sockets, a
  nose ridge, cheekbones, and a jaw with a glowing mouth that opens on a glowing maw. Long thick arms end in enormous
  fists that rest on the ground before it; short heavy legs set wide. Every moving part has its own pivot for the
  animations (§2.1): `body` (at the hips), `head` (the neck), `jaw`, `arm_*` (the shoulders), `forearm_*` (the
  elbows), `fist_*` (the wrists), `leg_*` (the hips), `shin_*` (the knees).

Next: the code once Horizon's launcher builds the pinned revision again (docs/HORIZON-ISSUES.md, #15764), and the
library's lobbed projectiles (common/SPECS.md §3.9) moved from the Cyclops with it.

## 1. Technical context

- **Mod id**: `magma_golem` ✅. Entity `magma_golem:magma_golem`, spawn egg `magma_golem:magma_golem_spawn_egg`, items
  `magma_golem:magma_heart`, `magma_golem:molten_pickaxe` ✅ (§5), projectile `magma_golem:magma_glob`.
- Uses from the library: model drawing and sounds (§3.1, §3.2), **hitbox size** (§3.4, a big mob), monsters sparing it
  (§3.4), the creature guide's pages (§7), and, moved from the Cyclops with this mod (rule of two), **the lobbed
  projectile** (the cyclops's boulder: lifted, thrown on an arc, breaking blocks where it lands or not;
  common/SPECS.md §3.9) ✅.
- It copies the wandering trader (goals only), a box of 3 × 6 blocks ✅ (6 blocks tall). Horizon: the boss bar
  (`Host::boss_bar`, shown to the players within 48 blocks).
- Its own sounds (grinding rock, a deep rumble, the hiss of lava) from freesound, CC0, cut by `cargo creature sounds`.

## 2. The magma golem ✅

- **Its look** (our own design): a hunched giant built from heavy boulders of dark rock, lava glowing orange in the
  seams between them; a small head held low in front, below its massive back and shoulders, two glowing eyes, long
  thick arms ending in enormous fists that rest on the ground, short heavy legs set wide. Its seams are
  drawn full-bright, and it lights its surroundings (light 12).
- **Dormant**: it sleeps **curled up as a mound of glowing rock** (a pose of the model: knees drawn up, arms around,
  head tucked, about 3 blocks high), still and silent, only its cracks slowly pulsing. The Nether has no day: it wakes
  when a player comes within 6 blocks, hits it or mines next to it, rising with a rumble (2 seconds) before it attacks.
  It lies down again where it stands after 2 minutes without a target, its health restored.
- **Spawning**: rare, in the **Nether**: basalt deltas and the nether wastes near lava seas, always dormant, on a
  floor with room for it (3 × 6 × 3), never within 256 blocks of another. It never despawns.
- **Fighting** (a mini-boss, its boss bar while awake): 250 health, armor 10, no knockback, immune to fire and lava.
  Slow heavy steps; it wades through lava and **heals in lava** (2 health a second).
  - **Fist slam**: in reach (4 blocks), 14 damage and sets on fire.
  - **Ground pound**: when players are within 6 blocks, both fists on the ground: a shockwave (8 damage, thrown up and
    back, set on fire), fire left around it.
  - **Magma glob**: at range (8 to 24 blocks), it tears a glob of magma from its body and lobs it on an arc (8 damage,
    a burst of fire where it lands, breaks no block).
  - **Enraged** under half health: its cracks blaze brighter, it moves faster, and each ground pound sheds 1 or 2
    small magma cubes.
- **Cooled by snow**: snowballs hurt it (3 damage) and a few in a row (5 within 4 seconds) **crust it over**: its cracks
  go dark for 6 seconds, it is slowed and takes double damage. A splash water bottle crusts it at once. The way to beat
  it in the Nether: bring snow.
- **Drops**: always its **magma heart** (§5), 4 to 8 magma cream, 2 to 6 magma blocks, 50 experience.

### 2.1 Animation ✅

Every movement is animated by the code from the model's parts (`creature::model::draw`'s `pose`), smoothed between
ticks, slow and heavy, the ground shaking under it (dust and rock particles, a deep thud).

- **Dormant**: the mound pose (legs folded under, back rounded high, arms wrapped around, head tucked under the
  brow), its eyes dark; only a slow breath lifts the back now and then.
- **Waking** (2 s): the back heaves first, the fists push on the ground, the legs unfold, the arms come out, the head
  rises last and its eyes light up, the jaw opening on a roar. **Lying down** is the same, backwards.
- **Idle**: a slow breath (the back and shoulders rise and fall), the head turning toward the nearest player, the
  fists shifting on the ground.
- **Walking**: heavy steps, one leg then the other with the knee bending, the body rolling from side to side and
  dipping on each footfall; the arms swing opposite the legs, its fists brushing the ground like an ape's.
- **Fist slam**: one arm rises high behind it (0.6 s), then smashes down in front (0.15 s), the body following the
  blow; it straightens again.
- **Ground pound**: both arms rise over its head (1 s, the warning), the jaw opens, both fists come down together, the
  body dropping with them.
- **Magma glob**: one hand tears a glob from the opposite shoulder, lifts it over the head and lobs it (the library's
  throw timing, common/SPECS.md §3.9).
- **Hurt**: a short flinch back, the head jerking.
- **Crusted** (cooled by snow): stiff and slow, every movement at half speed, its eyes dim.
- **Enraged**: a roar (the jaw wide open, the head thrown back, the arms spread), then every movement faster.
- **Death**: it drops to its knees, slumps forward on its fists and stays still as its glow fades.

## 3. Boss or not ✅

A mini-boss, found rather than summoned: rare and dormant in the Nether, a boss bar while it fights, a sure trophy
(its heart). Not a true boss like the Wither (no summoning, no altar, no world event): finding one sleeping is the
event. Summoning one (a ritual with magma blocks, as the iron golem or the Wither) could come later (§6).

## 4. Bestiary page ✅

The biomes met in, and the facts: "Awake" (woke one), "Crust" (cooled one with snow or water), "Lava bath" (saw one
heal in lava), "Heart" (took its heart).

## 5. Items ✅

- **Magma heart** (`magma_golem:magma_heart`): a glowing core of magma, warm in the hand. Its use:
- **Molten pickaxe** (`magma_golem:molten_pickaxe`): a netherite pickaxe and a magma heart in a smithing table. Mines
  as netherite, and **smelts what it mines** (iron, gold and copper ores give ingots, sand gives glass, stone gives
  stone, cobblestone gives stone), with the furnace's experience. Fireproof. Its durability is the netherite pickaxe's.

## 6. Out of scope for v1

- Summoning one, taming or riding it, smaller magma golems, a golem of another rock (an obsidian or a basalt golem),
  armour from its heart.

## 7. Open questions

- ~~Q1: The name?~~ Magma Golem, `magma_golem` (§1).
- ~~Q2: Where and how rare?~~ Basalt deltas and nether wastes near lava seas, 256 blocks apart (§2).
- ~~Q3: A boss?~~ A found mini-boss with a boss bar, no summoning in v1 (§3).
- ~~Q4: Its attacks and weakness?~~ Fist slam, ground pound, magma glob, enraged under half health; snow cools it (§2).
- ~~Q5: Its loot?~~ The magma heart, which makes the molten pickaxe (§5).
- ~~Q6: The thrown boulder?~~ Moved from the Cyclops to the library, shared with the magma glob (§1).
- ~~Q7: The animations?~~ Every movement animated, slow and heavy (§2.1).
