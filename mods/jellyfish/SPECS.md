# Jellyfish: specification

A mod of the **Sunrise Creatures** series for **Sunrise** (Minecraft Java 26.3): jellyfish drifting in blooms
through the oceans, pulsing slowly, trailing long tentacles that sting what brushes them. Some glow in the deep at
night.

**Requires** the series' library, Sunrise Creatures (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

Playable, tested in game (single player): everything below works, unless marked ❓.

- **The jellyfish** (`jellyfish:jellyfish`, spawn egg): its own model (`sources/models/jellyfish.bbmodel`, pictures in
  docs/models/), a texture per kind, its tentacles swaying and flaring out on each pulse. Its kinds are its looks,
  rolled by biome (§2).
- **Swimming** (§3): pulses up and sinks, drifting, 3 to 8 blocks under the surface by day, 1 to 3 by night (the glow
  jelly 8 to 16 by day), never out of the water.
- **Water life** through the library (common/SPECS.md §3.10): it never drowns; beached it falls, flops and dries out
  within a minute; a water bucket catches it into `jellyfish:jellyfish_bucket`, which lets it out again (its kind kept,
  never despawning). Blooms of 3 to 8 spawn in its oceans.
- **Stings** (§3, through the library's touches, common/SPECS.md §3.13), boats and turtle shells protecting; the glow
  jelly's light and full-bright glow; its drops and items (§5).
- **Blooms of one kind**: the kind is rolled once for a bloom, by its biome.
- **Turtles eat them** (§3, the library's hunters, common/SPECS.md §3.7): a sea turtle within 8 blocks swims to one
  and eats it ("Turtle" fact).
- **The glow jelly**, seen in a deep ocean at night: drawn full-bright, its light around it.
- **Sounds** (`sources/sounds.toml`, CC0 from freesound): a soft pulse as it swims up, a wet flop on land, a squish when
  hurt or dying, the slap of its sting; the bucket's are vanilla's fish sounds.

## 1. Technical context

- **Mod id**: `jellyfish` ✅. Entity `jellyfish:jellyfish` (one entity, the kinds as morphs, common/SPECS.md §7), spawn
  egg `jellyfish:jellyfish_spawn_egg`, items `jellyfish:jelly`, `jellyfish:glow_jelly`, `jellyfish:jellyfish_bucket`
  ✅ (§5).
- Uses from the library: model drawing and sounds (§3.1, §3.2), hitbox size (§3.4), monsters sparing it (§3.4), the
  creature guide's pages and morphs (§7), and water creatures (§3.10, planned, shared with the Flying Fish: breathing
  in water, beached, the bucket).
- It copies the wandering trader (goals only), with a box of 0.7 × 0.9 blocks ✅, spawned without gravity. Sunrise:
  the `living_breathe` hook lets it breathe in water and not in air (through the library, §3.10).
- Its own sounds (a soft pulse, a wet slap on land) from freesound, CC0, cut by `cargo creature sounds`.

## 2. Kinds ✅

| Kind | Where | Sting |
| --- | --- | --- |
| **Moon jelly** | oceans, lukewarm and warm oceans | mild: 1 damage |
| **Sea nettle** | lukewarm and warm oceans | 2 damage and Poison 4 s |
| **Glow jelly** | deep oceans, by night only near the surface | 1 damage and Glowing 10 s; it glows (drawn full-bright) |

## 3. The jellyfish ✅

- About half a block across the bell, its tentacles trailing a block below. 6 health. Never attacks.
- **Spawning**: in the water of its biomes, in **blooms** of 3 to 8 of one kind. Wild ones despawn far from players.
- **Swimming**: it pulses: the bell contracts and it rises about a block, then it sinks slowly, drifting; flowing
  water carries it. It stays under the surface. By night the blooms rise toward the surface, by day they sink deeper.
  It never chases anything.
- **Stings**: a player or mob touching it is stung (its kind's sting, §2), at most once a second. Players in a boat are
  safe, and so is a player wearing a turtle shell.
- **Beached**: out of water it can't move, flops now and then, dries out within a minute and dies (it drops as usual).
  It still stings whoever walks into it.
- **Turtles eat them**: a sea turtle that sees a jellyfish within 8 blocks swims to it and eats it (no drop), as real
  sea turtles do. Through the library's prey (common/SPECS.md §3.7), the turtle as a second hunter beside the chicken.
- **Drops**: 0 to 2 jelly (glow jelly from a glow jelly).

## 4. Bestiary page ✅

The kinds met as its morphs, and the facts: "Bloom" (saw 5 together), "Sting" (stung by one), "Glow" (saw a glow jelly
at night), "Bucket" (caught one in a bucket), "Turtle" (saw a turtle eat one).

## 5. Items ✅

- **Jelly** (`jellyfish:jelly`): a translucent blob; crafts into a slime ball (shapeless, one for one).
- **Glow jelly** (`jellyfish:glow_jelly`): a glowing blue blob; crafts into a glow ink sac (one for one).
- **Bucket of jellyfish** (`jellyfish:jellyfish_bucket`): a water bucket used on a jellyfish catches it, as a fish; it
  is let out again where the bucket is emptied, its kind kept, to fill an aquarium. A caught one never despawns. Its
  icon is the game's own water bucket (its model's first layer) with the jellyfish in it (ours, the second layer). In
  the creative tab; that one lets out a jellyfish of the kind its spot gives.

## 6. Out of scope for v1

- Befriending, breeding and polyps, the glass jar (it needs water), other kinds (box jellyfish, lion's mane, Portuguese
  man o' war), cooking jellyfish.

## 7. Open questions

- ~~Q1: The kinds?~~ Moon jelly, sea nettle, glow jelly, each with its sting (§2).
- ~~Q2: Swimming?~~ In pulses, drifting, rising at night (§3).
- ~~Q3: The turtle shell?~~ It protects from stings (§3).
- ~~Q4: Turtles?~~ They eat jellyfish, through the library's prey (§3).
- ~~Q5: The items?~~ Jelly as slime balls, glow jelly as glow ink, the bucket (§5).
