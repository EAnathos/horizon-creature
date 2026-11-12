# Cyclops: specification (v0.1, draft)

A mod of the **Horizon Creature** series for **Minecraft Horizon** (Minecraft Java 26.3): the **cyclops**, a one-eyed
giant of the mountains that hurls boulders, and whose eye is a rare crafting material ✅.

**Requires** the series' library, Horizon Creature (`creature`, [common/SPECS.md](../../common/SPECS.md)).
**Used by** Wyverns when both are installed ✅: the Wyvern Staff then takes a cyclops eye in place of the eye of ender
(Wyverns §2). Neither mod requires the other.

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Progress (as of 2026-11-09)

- Folder and spec created.
- **Model** (2026-11-12): `assets/models/cyclops.bbmodel`, 31 cubes, texture 128×128 (docs/MODELS.md): 4 blocks
  tall, blue skin, one big amber eye under a heavy brow, tusks, a hide loincloth, a knotted wooden club in its right
  hand (group `club`, child of `arm_right`). Groups for posing: `leg_*`, `body`, `head`, `arm_*`, `club`. No content or
  native half yet: the logic comes later.

## 1. Technical context

- **Mod id**: `cyclops`. Entity `cyclops:cyclops`, items `cyclops:cyclops_eye` ✅, `cyclops:cyclops_spawn_egg`;
  `cyclops:boulder` (projectile ❓).
- Uses from the library: model drawing and sounds (§3.1, §3.2), **hitbox size** (§8.8, a big mob), **ranged attack
  goal** (§8.7, the boulders), the creature guide's pages later (§9).
- A hostile mob: it copies the zombie ❓ (hostile goals) or the trader with our own target goals ❓.

## 2. The cyclops ❓

- A **giant**, **4 blocks tall** ✅ (hitbox ~2 × 4 ❓): **blue skin** ✅, a **single huge eye** ✅ in the middle of its
  brow, a heavy jaw with tusks, a loincloth of hide, a **wooden club** ✅.
- **Neutral until provoked** ❓, or hostile on sight within ~16 blocks ❓. Slow, heavy steps that shake the screen a
  little ❓.
- **Attacks** ❓:
  - **club smash**: a slow overhead blow (12 damage ❓) that knocks back;
  - **boulder throw**: at range (8 to 24 blocks ❓), it tears a boulder from the ground and hurls it in an arc (10 damage
    ❓, breaks no blocks ❓).
- **Its eye is its weak spot** ❓: an arrow or a trident in the eye deals double damage and stuns it for 2 s ❓.
- Health 80 ❓, armour 4 ❓, knockback resistance 0.6 ❓.
- **Spawning** ❓: rare, alone, in mountain biomes (peaks, slopes, stony shores) and windswept hills; in a cave
  entrance it guards a small hoard ❓.

## 3. Cyclops eye ✅

- `cyclops:cyclops_eye`: dropped when a cyclops is killed (1 ✅ ❓; Looting doesn't add any ❓).
- **Wyvern Staff** ✅: with Wyverns installed, it replaces the eye of ender in the staff's recipe (Wyverns §2).
- Other uses ❓: a brewing ingredient (a potion of night vision with a longer time ❓), or nothing else in v1.

## 4. Out of scope for v1

- Taming or riding a cyclops, cyclops villages, a cyclops boss.

## 5. Open questions

- Q1: Neutral or hostile? Its size, health and attacks (§2).
- Q2: Where it spawns and how rare it is.
- Q3: The eye's drop rate and any use besides the Wyvern Staff (§3).
