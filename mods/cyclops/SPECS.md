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
  hand (group `club`, child of `arm_right`). Groups for posing: `leg_*`, `body`, `head`, `arm_*`, `club`.
- **The cyclops in game** (2026-11-12, tested): `cyclops:cyclops` (a copy of the wandering trader, box 1.8 × 4, 80
  health), its spawn egg, the cyclops eye (dropped when a player kills it, with 1 to 3 leather), English and French
  names, a bestiary page (the Flock fact). Hostile (2026-11-12): it goes for a player it sees within 16 blocks, and
  for whoever hurts it or one of its sheep: it walks up and brings its club down from over its head (12 damage, a strong push), and lets go 32 blocks away. Heavy
  steps and a grumble (a ravager's sounds, lower, until it has its own). **Spawning**: rarely, at a cave mouth (open
  ground at the foot of a covered passage into a hillside) 24 to 64 blocks from a player, never within 160 blocks of
  another cyclops, with a **flock of 3 to 5 sheep** (tagged `cyclops.flock`) that follow it (tested: it appeared at a
  cave mouth with 5 sheep; its sheep walked after it; hitting one made it attack).
- **Shepherd's crook and club** (2026-11-12, tested): `cyclops:shepherds_crook` (a cyclops eye, 4 sticks and wool):
  while a player holds it, in either hand, the farm animals within 12 blocks follow them (sheep, cows, mooshrooms,
  pigs, goats, chickens), a cyclops's sheep too. `cyclops:club`: a heavy wooden weapon (8 damage, slow, extra
  knockback, 250 uses), crafted from 4 logs and a stick, and dropped worn by a cyclops a player kills (1 in 3). Neither can
  be enchanted (no `enchantable`; tested at a table). The eye has its own icon, and drops only when a player kills the
  cyclops (the loot's condition fixed to 26.3's form).
- Not yet: the boulder throw, its eye as a weak spot, its own sounds.

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
- **Wyvern Staff** ✅: with Wyverns installed, it replaces the eye of ender in the staff's recipe (Wyverns §2).
- **Shepherd's crook** ✅: crafted with it (§0). Other uses ❓ (ideas: an all-seeing lantern, a boulder sling, a
  potion of keen sight).

## 4. Out of scope for v1

- Taming or riding a cyclops, cyclops villages, a cyclops boss.

## 5. Open questions

- ~~Q1: Neutral or hostile?~~ Hostile. Its size, health and attacks (§2) still to balance.
- Q2: Where it spawns and how rare it is.
- Q3: The eye's drop rate and any use besides the Wyvern Staff (§3).
