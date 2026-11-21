# Flying Fish: specification

A mod of the **Sunrise Creatures** series for **Sunrise** (Minecraft Java 26.3): shoals of flying fish in warm
seas that burst out of the water and glide over the waves on their wing-like fins, to flee a dolphin, a boat or a
swooping seagull.

**Requires** the series' library, Sunrise Creatures (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

Playable (0.1.0): the three kinds in shoals of one kind, swimming as a shoal, gliding when frightened (a player, a hit, a
dolphin or a boat) or now and then, bumping off what they glide into, beached and picked up by hand, the bucket, raw and
cooked flying fish (fished with a rod in their oceans, in the `minecraft:fishes` tag, food for cats), dolphins hunting
them and fed by a catch, its own sounds, the guide's page with its facts. Played in game.

- **The model** (`sources/models/flying_fish.bbmodel`, pictures in docs/models/): a slim fish about three quarters of
  a block long, its big pectoral fins spread as wings (about 0.8 block across, each in two parts swept back), a small
  pair of pelvic fins behind them (the "four-winged" look), a dorsal and an anal fin, and a forked tail whose lower lobe
  is longer, as the real fish's. Groups: `body`, `fin_left`, `fin_right` (folded along the body by the pose while
  swimming, spread to glide), `pelvic_left`, `pelvic_right`, `tail`, `tail_fin`. One model for every kind, a texture
  each (§2): blue wing (navy back, silver belly, pale blue wings), spotted wing (olive back, pale wings with dark
  spots), sunset wing (blue back, orange wings, gold tips, a dark red edge).
- Seagulls (with the Seagulls): they snatch gliding flying fish, and frighten a shoal. The guide's "Seagull" fact is ❓
  (§4).

## 1. Technical context

- **Mod id**: `flying_fish` ✅. Entity `flying_fish:flying_fish` (one entity, the kinds as morphs, common/SPECS.md §7),
  spawn egg `flying_fish:flying_fish_spawn_egg`, items `flying_fish:raw_flying_fish`, `flying_fish:cooked_flying_fish`,
  `flying_fish:flying_fish_bucket` ✅ (§5).
- Uses from the library: model drawing and sounds (§3.1, §3.2), hitbox size (§3.4), monsters sparing it (§3.4), the
  creature guide's pages and morphs (§7), hunters (§3.7: dolphins, and seagulls when the Seagulls are installed, which add themselves as its hunters), and **water creatures** (§3.10, with
  the Jellyfish: breathing in water, drying out when beached, the bucket).
- It copies the wandering trader (goals only), with a box of 0.5 × 0.3 blocks ✅, without gravity in water. Sunrise:
  the `living_breathe` hook lets it breathe in water and not in air (through the library, §3.10).
- Its own sounds (a splash on leaving and entering the water, a buzz of fins while gliding, flopping on land) from
  freesound, CC0, cut by `cargo creature sounds` ✅ (`sources/sounds.toml`).

## 2. Kinds ✅

| Kind | Where | Share |
| --- | --- | --- |
| **Blue wing** | lukewarm, deep lukewarm and warm oceans | common |
| **Spotted wing** | lukewarm and deep lukewarm oceans | common |
| **Sunset wing** | warm oceans | rare (one in 10 there) |

A shoal is of one kind.

## 3. The flying fish ✅

- About three quarters of a block long. 3 health. Never attacks.
- **Spawning**: in the water of its biomes, near the surface, in **shoals** of 4 to 10, at any hour. Wild ones despawn
  far from players.
- **Swimming**: it swims as a shoal in the top 3 blocks of the water, its fins folded along its body, turning together
  with its shoal as vanilla's fish school.
- **Gliding**, its signature: when frightened (a player, a boat, a dolphin or a seagull within 6 blocks, a hit), and now and then
  for no reason, it races to the surface and **bursts out of the water**: it spreads its fins and **glides** a block or
  two above the waves, at about a block a tick, for 10 to 20 blocks, then beats its tail on a
  wave to glide again (up to 2 times, so up to about 50 blocks), and dives back in with a splash. A frightened shoal
  takes off nearly together, fanning out. Rain or a storm cuts its glides short. It never glides over land on purpose
  and turns back toward open water.
- **Beached** (the library's water creatures, §3.10): out of water it can't swim, it flops toward the water now and
  then and dries out within 30 s, then dies (it drops as usual). A player picks one up with a right-click (raw flying
  fish) or catches it alive in a water bucket.
- **Hunted**: dolphins chase and eat flying fish (the library's prey, §3.7): a dolphin that sees one within 12 blocks
  swims after it, and the shoal takes off. A dolphin that caught one is happy for a while (it leads players to treasure
  as if fed: its `GotFish`) ✅.
- **Seagulls** ✅, when the Seagulls mod is installed (no requirement either way): a seagull flying over the sea that sees a
  gliding flying fish within 16 blocks swoops and **snatches it in the air** (it is gone, no drop). A
  befriended seagull fishing for its friend (mods/seagulls/SPECS.md §3) may bring back a raw flying fish in their
  biomes. Through the library's prey (common/SPECS.md §3.7), with a mod's mob as the hunter.
- **Hitting it** ✅: a gliding fish that hits a player or a mob (within half a block of its middle) bumps off (no damage)
  and falls into the water, or on the ground, where it is beached. A glider that runs into a block drops.
- **Drops**: 1 raw flying fish (cooked when it burns).

## 4. Bestiary page ✅

The kinds met as its morphs, and the facts: "Glide" (saw one glide more than 30 blocks), "Shoal" (saw a shoal take off
together), "Dolphin" (saw a dolphin catch one), "Rod" (caught one with a fishing rod), "Bucket" (caught one in a
bucket); "Seagull" (saw a seagull snatch one) comes with the Seagulls ❓.

## 5. Items ✅

- **Raw flying fish** and **cooked flying fish** (`flying_fish:raw_flying_fish`, `flying_fish:cooked_flying_fish`):
  ordinary fish like cod (raw 2 hunger, cooked 5), cooked in a furnace, a smoker or on a campfire. In the `minecraft:fishes`
  item tag, so it counts as a fish wherever vanilla takes any fish from that tag (both), and the raw one in
  `minecraft:cat_food` and `minecraft:ocelot_food`: cats and ocelots take it as cod (additions to vanilla's tags,
  which Sunrise merges as a data pack's).
- **Bucket of flying fish** (`flying_fish:flying_fish_bucket`): a water bucket used on a flying fish catches it, as a
  fish (the library's bucket, §3.10); it is let out where the bucket is emptied, its kind kept. A caught one never
  despawns.
- Fishing with a rod in its biomes sometimes brings up a raw flying fish ✅: a cod caught there is one two times in five,
  about as often as a salmon (the `item_fished` hook: a mod can't replace vanilla's fishing loot table).

## 6. Out of scope for v1

- Breeding and eggs, other kinds (the two-winged and four-winged species beyond the three textures), riding on a glide,
  a flying fish trophy or roe, being drawn to lights at night (left out, Q3).

## 7. Open questions

- ~~Q1: The mod id?~~ `flying_fish`, singular as the species (§1).
- ~~Q2: The kinds?~~ Blue, spotted and sunset wings, one shoal one kind (§2).
- ~~Q3: Drawn to lights at night, landing on deck?~~ No: they glide only when frightened or now and then (§3, §6).
- ~~Q4: Fishing loot?~~ Yes, in their oceans as often as a salmon (§5).
- ~~Q5: Seagulls catching them?~~ Yes, through the library's prey with mod hunters, when both mods are installed (§3).
