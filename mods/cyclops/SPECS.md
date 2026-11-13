# Cyclops: specification

A mod of the **Horizon Creature** series for **Minecraft Horizon** (Minecraft Java 26.3): the **cyclops**, a one-eyed
giant of the mountains that hurls boulders, and whose eye is a rare crafting material ✅.

**Requires** the series' library, Horizon Creature (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

Done and tested in game (single player).

- **The cyclops** (`cyclops:cyclops`, spawn egg): a copy of the wandering trader, 4 blocks tall (box 1.8 × 4), 80
  health, blue, one big eye, tusks, a hide loincloth and a wooden club (`assets/models/cyclops.bbmodel`).
- **Hostile** (§2): it goes for a player it sees within 16 blocks, and for whoever hurts it or one of its sheep. In
  reach, it raises its club over its head and brings it down (12 damage, a strong push); out of reach (7 to 24
  blocks), it lifts a **boulder** over its head and hurls it on an arc (`cyclops:boulder`, 10 damage, a push; it
  shatters on a hit, the ground or a wall, and breaks no blocks). It lets go 32 blocks away.
- **Its eye is its weak spot**: an arrow or a trident in it deals double damage and dazes it for 2 s.
- **Spawning**: rarely, at a cave mouth 24 to 64 blocks from a player, never within 160 blocks of another cyclops,
  with a **flock of 3 to 5 sheep** that follow it. Biomes: meadows, cherry groves, groves, snowy slopes, stony peaks,
  the windswept biomes, savanna plateaus and stony shores.
- **Drops**: its eye when a player kills it, sometimes its worn club (1 in 3), 1 to 3 leather.
- **Shepherd's crook** (§3): a cyclops eye, 4 sticks and wool; while a player holds it, the farm animals within 12
  blocks follow them. **Club**: a heavy wooden weapon (8 damage, slow, extra knockback, 250 uses), from 4 logs and a
  stick. Neither can be enchanted, at a table or an anvil.
- **Bestiary page**: the biomes met in, "Flock".

Left: its own sounds (a ravager's, lower, until then; picked from freesound, not made yet); balance as players report.

## 1. Technical context

- **Mod id**: `cyclops`. Entity `cyclops:cyclops`, items `cyclops:cyclops_eye` ✅, `cyclops:cyclops_spawn_egg`;
  `cyclops:boulder` (projectile ❓).
- Uses from the library: model drawing and sounds (§3.1, §3.2), **hitbox size** (§8.8, a big mob), **ranged attack
  goal** (§8.7, the boulders), the creature guide's pages later (§9).
- A hostile mob: it copies the zombie ❓ (hostile goals) or the trader with our own target goals ❓.

## 2. The cyclops ❓

- A **giant**, **4 blocks tall** ✅ (hitbox ~2 × 4 ❓): **blue skin** ✅, a **single huge eye** ✅ in the middle of its
  brow, a heavy jaw with tusks, a loincloth of hide, a **wooden club** ✅.
- **Hostile** ✅: it goes for a player it sees within ~16 blocks, and for whoever hurts it or one of its sheep ✅ (a
  shepherd, as Polyphemus). Slow, heavy steps that shake the screen a
  little ❓.
- **Attacks** ❓:
  - **club smash**: a slow overhead blow (12 damage ❓) that knocks back;
  - **boulder throw**: at range (8 to 24 blocks ❓), it tears a boulder from the ground and hurls it in an arc (10 damage
    ❓, breaks no blocks ❓).
- **Its eye is its weak spot** ❓: an arrow or a trident in the eye deals double damage and stuns it for 2 s ❓.
- Health 80 ❓, armour 4 ❓, knockback resistance 0.6 ❓.
- **Spawning** ✅: rare, alone, **at a cave mouth**, with a **flock of sheep that follow it** ✅. Biomes ❓: meadows,
  groves, snowy slopes, stony peaks, windswept hills, gravelly hills, forests and savannas, savanna plateaus, plains and
  taigas. It could guard a small hoard in the cave ❓.

## 3. Cyclops eye ✅

- `cyclops:cyclops_eye`: dropped when a cyclops is killed (1 ✅ ❓; Looting doesn't add any ❓).
- **Shepherd's crook** ✅: crafted with it (§0). Other uses ❓ (ideas: an all-seeing lantern, a boulder sling, a
  potion of keen sight).

## 4. Out of scope for v1

- Taming or riding a cyclops, cyclops villages, a cyclops boss.

## 5. Open questions

- ~~Q1: Neutral or hostile?~~ Hostile. Its size, health and attacks (§2) still to balance.
- Q2: Where it spawns and how rare it is.
- Q3: The eye's drop rate (§3).
