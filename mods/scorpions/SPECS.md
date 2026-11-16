# Scorpions: specification

A mod of the **Sunrise Creatures** series for **Minecraft Horizon** (Minecraft Java 26.3): scorpions of real size,
hiding in the shade of deserts, badlands and caves. Come close and one rears up in a defensive posture, claws open and
tail arched over its back; leave it be and it backs away. Hit it and it fights back, pinching and stinging with poison.

**Requires** the series' library, Sunrise Creatures (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

In development: the spec, the model, the code (native and content), and a first pass of textures and sounds. It builds,
installs and loads in game (its items, the creative tab and the entity are in, checked 2026-11-16); still to balance and
to replace the placeholder art. The textures are **placeholders** (painted from the model's UVs,
a colour per body part) and the sounds are **synthesized placeholders** (not the freesound clips `cargo creature
sounds` cuts, so there is no `assets/sounds.toml`): both are meant to be replaced by real art.

- **The model** (`assets/models/scorpion.bbmodel`, pictures in docs/MODELS.md): a carapace with a pair of eyes on top
  and small ones on its front corners, three broad body segments, eight thin legs bent at the knee, two big claws
  (pedipalps: arm, forearm, a palm with a fixed finger and a moving finger, `claw_*_finger`, that opens and closes),
  and the tail in five segments ending in the bulb and its hooked stinger (`tail_1` to `tail_5`, `telson`). Built at a
  readable size (about a block with its claws, 16 texels) and **drawn small** (§1); built with its tail curled over
  its back, which the pose raises higher in the defensive posture or lowers when it walks. A texture per kind (§2):
  desert (pale straw, dark amber stinger), badlands (red-brown), cave (charcoal, its segments' edges glowing cyan, as
  real scorpions glow under ultraviolet light).
- **The code**: one entity, the kinds as morphs (the library's colours, `SCHEME`), crawling on the ground (the
  library's small crawlers), the defensive posture (`posture.rs`, backing away and the step-on sting), fighting back
  (`fight.rs`, the vanilla "hurt by" target goal and a poisoned sting), the jar and the sand rule, the bestiary page,
  and the content (entity, spawn egg, the venom sac, stinger and scorpion dagger with their recipes and the venom-sac
  brewing, the loot table, the biome spawns, en/fr).

Left to do: **choose the scorpion's morph when using `/summon`** (§3; implement and test), and the dagger's Poisonous I
enchantment and throwing (§5; neither works yet and needs implementation); real art to replace the placeholder
creature textures and remaining item icons; the synthesized sounds (then, for sounds from freesound, add
`assets/sounds.toml` and cut them with `cargo creature sounds`); **balance**. To refine: the cave kind by depth rather
than biome (§2), the pinch's and sting's damage (the wandering-trader template has no `attack_damage` attribute, so
`do_hurt_target` deals the engine default for both; the sting adds poison), and stinging the player who tries to catch a
reared scorpion (§6).

## 1. Technical context

- **Mod id**: `scorpions` ✅. Entity `scorpions:scorpion` (one entity, the kinds as morphs, common/SPECS.md §7), spawn egg
  `scorpions:scorpion_spawn_egg`, items `scorpions:venom_sac` and `scorpions:stinger` ✅ (§5). A caught one is in the
  library's `creature:glass_jar` ✅ (§6).
- Uses from the library: model drawing and sounds (§3.1, §3.2), hitbox size (§3.4, tiny), monsters sparing it (§3.4),
  **small crawlers** (§3.6), the **glass jar and territories** (§4, which this mod switches on; rule "sand"), the
  creature guide's pages and morphs (§7).
- It copies the wandering trader (goals only), with a box of 0.25 × 0.15 blocks ✅, its model drawn at a fifth of its
  size ✅ (about 0.2 block long, claws to the tip of the curled tail: a large real scorpion).
- **Real size** ✅: as small as in real life, so easy to miss. Its attacks come from its own goals (the wandering
  trader has none), and its poison through `LivingEntity` effects.
- Its own sounds (a dry hiss when it threatens, as some scorpions stridulate, the snap of its claws, a skitter when it
  runs) from freesound, CC0, cut by `cargo creature sounds`.

## 2. Kinds ✅

| Kind | Where | Its poison |
| --- | --- | --- |
| **Desert** (pale straw) | deserts | Poison 5 s |
| **Badlands** (red-brown) | badlands, wooded and eroded badlands | Poison 5 s |
| **Cave** (charcoal, glowing cyan edges, drawn full-bright) | caves under deserts and badlands, dripstone caves, below y 40 | Poison 8 s and Nausea 4 s |

## 3. The scorpion ✅

- 4 health. Neutral: it never starts a fight.
- **Spawning**: in its biomes, alone or 2 together, on sand, red sand, terracotta, sandstone and stone; the cave
  kind in the dark. Wild ones despawn far from players.
- **Summoning**: `/summon scorpions:scorpion` must let the player choose its morph (desert, badlands or cave); the
  selected kind must be saved on the scorpion.
- **Life**: nocturnal. By day it shelters in the shade (under an overhang, beside a dead bush or a cactus, in a
  cave mouth) and keeps still; at dusk it comes out and wanders slowly (the library's small crawlers), its tail low.
  The cave kind is about at any hour.
- **Defensive posture** ✅, its signature: a player (or a mob that is not ours) within **3 blocks** ✅ makes it stop,
  turn to face them and **rear up**: claws raised and open toward them, tail arched high over its back with the stinger
  poised, hissing now and then. It keeps facing them and **backs away** slowly. A sneaking player can come to 1.5
  blocks ✅ before it rears up. Once they are 5 blocks away it lowers its tail and goes on.
- **It fights back** ✅: hit (by a player, an arrow, a mob), it **attacks whoever hit it**: it runs at them and
  strikes about once a second, a **pinch** with a claw (1 damage) or, one strike in 2, a **sting** (2 damage and its
  kind's poison, §2). It chases up to 12 blocks ✅ and calms down after 10 s ✅ without being hit again, back in its
  posture.
- **Stepped on** ✅: a player or a mob walking into it (its box touching the scorpion's) is **stung once** (its
  kind's sting, at most once every 2 s), without a chase: it rears up and stays in its posture.
- **Drops**: 0 to 1 venom sac and, one in 3, its stinger (§5).

## 4. Bestiary page ✅

The kinds met as its morphs, and the facts: "Posture" (saw one rear up), "Stung" (stung by one), "Glow" (saw a cave
scorpion glow), "Jar" (caught one in a jar), "Settled" (settled one in a sandy patch).

## 5. Items ✅

- **Venom sac** (`scorpions:venom_sac`) ✅: a small amber sac. Uses ✅: brewed in an awkward potion it makes a potion
  of Poison, as a spider eye does (if Horizon lets a mod add a brewing recipe, to check when implementing); 8 arrows
  around a venom sac craft 8 arrows of Poison (vanilla's tipped arrows).
- **Stinger** (`scorpions:stinger`) ✅: the hooked stinger. Use ✅: with a stick, crafts a **scorpion dagger**
  (`scorpions:scorpion_dagger`): a quick, light blade (3 damage, faster than a sword), 64 uses. **Wanted behavior, not
  implemented yet**: it should visibly have the **Poisonous I** enchantment and be throwable. Neither the enchantment
  display nor throwing currently works. **TODO for Claude: implement both.** A dagger hit should poison its target for
  3 s.

## 6. Glass jar and territory ✅

- **Caught** in the library's glass jar ✅ (common/SPECS.md §4.1), its kind and name kept. One in its defensive posture
  or angry stings the player who tries (and is caught anyway) ✅; a sneaking player catches it safely. On display, a
  jar shows it with its tail up.
- **Released on sand it settles** (the library's rule **sand**, §4.2, new with this mod) ✅: released on sand, red sand,
  sandstone or terracotta, its patch is those blocks connected to it within 8 blocks, at least 10 of them, otherwise it
  won't settle. A settled one stays on its patch and never despawns: a terrarium. It still rears up at players who come
  close, and fights back if hit.
- The jar's recipe condition and the tab injection get `scorpions` with this mod (library §3.3, §4.1).

## 7. Out of scope for v1

- Breeding (a mother carrying her young on her back), giant scorpions, other kinds (the emperor scorpion of jungles),
  burrows dug in sand, riding, hunting the series' small creatures (worms, dung beetles, phasmids).

## 8. Open questions

- ~~Q1: Size?~~ Real size, as small as in real life (§1).
- ~~Q2: Kinds?~~ Desert, badlands and a glowing cave kind (§2).
- ~~Q3: What it gives?~~ A venom sac, a stinger, and catching it in the glass jar (§5, §6).
- ~~Q4: Behaviour?~~ A defensive posture when someone comes close; it fights back with claws and poison when hit (§3).
- ~~Q5: Distances?~~ Posture at 3 blocks, 1.5 for a sneaking player, chase up to 12 blocks, calm after 10 s (§3).
- ~~Q6: Stepping on it?~~ One sting, no chase (§3).
- ~~Q7: The venom sac?~~ A potion of Poison and poison arrows (§5).
- ~~Q8: The stinger?~~ The scorpion dagger (§5).
- ~~Q9: The jar?~~ It stings a catcher who isn't sneaking, and settles on a sandy patch (§6).
- ~~Q10: Hunting small creatures?~~ Out of scope for v1 (§7).
