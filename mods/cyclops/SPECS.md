# Cyclops: specification

A mod of the **Horizon Creature** series for **Minecraft Horizon** (Minecraft Java 26.3): the **cyclops**, a one-eyed
giant of the mountains that hurls boulders, and whose eye is a rare crafting material ✅.

**Requires** the series' library, Horizon Creature (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

Done and tested in game (single player), except the blocks its blows break (built, not yet checked in game).

- **The cyclops** (`cyclops:cyclops`, spawn egg): a copy of the wandering trader, 4 blocks tall (box 1.8 × 4), 110
  health, blue, one big eye, tusks, a hide loincloth and a wooden club (`assets/models/cyclops.bbmodel`).
- **Hostile** (§2): it goes for a player it sees within 16 blocks, and for whoever hurts it or one of its sheep. In
  reach (its target's feet less than 6 blocks above its own), it raises its club over its head and brings it down (12 damage, a strong push); out of reach (7 to 24
  blocks), it lifts a **boulder** over its head and hurls it on an arc (`cyclops:boulder`, 10 damage, a push; it
  shatters on a hit, the ground or a wall); a target up out of the club's height gets a boulder at any distance. Both tear up the blocks around where they land, without drops (`smash`):
  a ball of radius 1 for the club, 2 for the boulder, sparing liquids, containers, placed jars and the blocks players
  can't break, and nothing at all with the `mob_griefing` game rule off. It lets go 32 blocks away.
- **Its eye is its weak spot**: an arrow or a trident in it deals double damage and dazes it for 2 s.
- **Spawning**: never on Peaceful (those there go, as vanilla's monsters); rarely, at a cave mouth 24 to 64 blocks from a player, never within 160 blocks of another cyclops,
  with a **flock of 3 to 5 sheep** that follow it. Biomes: meadows, cherry groves, groves, snowy slopes, stony peaks,
  the windswept biomes, savanna plateaus and stony shores.
- **Drops**: its eye when a player kills it, sometimes its worn club (1 in 3), 1 to 3 leather.
- **Shepherd's crook** (§3): a cyclops eye, 4 sticks and wool; while a player holds it, the farm animals within 12
  blocks follow them. **Club**: a heavy wooden weapon (8 damage, slow, extra knockback, 250 uses), from 4 logs and a
  stick. Neither can be enchanted, at a table or an anvil.
- **Bestiary page**: the biomes met in, "Flock".

- **Sounds**: its own, from CC0 freesound recordings (`assets/sounds.toml`): grunts, heavy steps, hurt and death, a
  roar as it lifts a boulder, the club's swing and thud, the throw, the boulder shattering, a dazed groan.

Left: balance as players report.

## 1. Technical context

- **Mod id**: `cyclops`. Entity `cyclops:cyclops`, items `cyclops:cyclops_eye` ✅, `cyclops:cyclops_spawn_egg`;
  `cyclops:boulder` (projectile ✅).
- Uses from the library: model drawing and sounds (§3.1, §3.2), **hitbox size** (§3.4, a big mob) and the creature
  guide's pages (§7). The boulder throw is the cyclops's own goal today; it moves to the library with the Magma Golem (common/SPECS.md §3.9, lobbed projectiles).
- A hostile mob ✅: it copies the wandering trader, with our own goals and target goals.

## 2. The cyclops ✅

- A **giant**, **4 blocks tall** ✅ (hitbox 1.8 × 4 ✅): **blue skin** ✅, a **single huge eye** ✅ in the middle of its
  brow, a heavy jaw with tusks, a loincloth of hide, a **wooden club** ✅.
- **Hostile** ✅: it goes for a player it sees within ~16 blocks, and for whoever hurts it or one of its sheep ✅ (a
  shepherd, as Polyphemus). Slow, heavy steps ✅.
- **Attacks** ✅:
  - **club smash**: a slow overhead blow (12 damage) that knocks back and breaks the blocks around where it comes down
    ✅;
  - **boulder throw**: at range (7 to 24 blocks), it lifts a boulder over its head and hurls it in an arc (10 damage,
    and it tears up the blocks around where it lands ✅).
- **Its eye is its weak spot** ✅: an arrow or a trident in the eye deals double damage and dazes it for 2 s.
- Health 110 ✅, knockback resistance 0.6 ✅.
- **Spawning** ✅: rare, alone, **at a cave mouth**, with a **flock of sheep that follow it** ✅. Biomes ✅: meadows, cherry
  groves, groves, snowy slopes, stony peaks, the windswept biomes, savanna plateaus and stony shores.

## 3. Cyclops eye ✅

- `cyclops:cyclops_eye`: dropped when a cyclops is killed (1 ✅; Looting doesn't add any ✅).
- **Shepherd's crook** ✅: crafted with it (§0). No other use ✅.

## 4. Out of scope for v1

- Taming or riding a cyclops, cyclops villages, a cyclops boss.

## 5. Open questions

- ~~Q1: Neutral or hostile?~~ Hostile. Its size, health and attacks (§2) still to balance.
- ~~Q2: Where it spawns and how rare it is?~~ At a cave mouth, rarely, far from other cyclops (§0).
- ~~Q3: The eye's drop rate?~~ One, when a player kills it (§3).
