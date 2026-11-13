# Seagulls: specification

A mod of the **Horizon Creature** series for **Minecraft Horizon** (Minecraft Java 26.3): noisy gulls of beaches and
harbours that glide along the coast, bob on the waves, follow boats, and snatch food from careless hands. Fed fish,
one becomes a fishing companion that dives for fish and shows you where to cast.

**Requires** the series' library, Horizon Creature (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

Planned: the spec and the model, no code yet.

- **The model** (`assets/models/seagull.bbmodel`, pictures in docs/MODELS.md): a white gull with a pale grey back and
  wings, black wing tips with white spots, a yellow hooked beak with a red spot, pale eyes and pink webbed feet. Its
  parts are named as the vulture's (`body`, `tail`, `neck`, `head`, `wing_*`, `wing_*_mid`, `wing_*_tip`, `leg_*`), so
  the shared flight poses it; built with its wings spread, folded on the ground by the pose.

Next: the code once Horizon's launcher builds the pinned revision again
(docs/HORIZON-ISSUES.md, #15764). The library's side comes with it: the vulture's flight (without riding) and taming
move to the library (common/SPECS.md §6.1, §6.2).

## 1. Technical context

- **Mod id**: `seagulls` ✅. Entity `seagulls:seagull`, spawn egg `seagulls:seagull_spawn_egg`, items
  `seagulls:feather`, `seagulls:raw_seagull`, `seagulls:cooked_seagull` ✅ (§5).
- Uses from the library: model drawing and sounds (§3.1, §3.2), hitbox size (§3.4), monsters sparing it (§3.4), the
  creature guide's pages (§7), and, moved from the vulture with this mod (rule of two): **flight** (the glide, soaring
  in circles, landing; not riding, §6.1) and **taming and ownership** (§6.2, with the Sporelins).
- It copies the wandering trader (goals only), with a box of 0.6 × 0.6 blocks ✅. Its own sounds (the gull's cry,
  a laughing call) from freesound, CC0, cut by `cargo creature sounds`.

## 2. The seagull ✅

- A herring gull, about two thirds of a block long, wings about 2.5 blocks across. 8 health. Neutral: it never attacks.
- **Spawning** ✅: beaches, stony shores, snowy beaches, and the coasts of oceans and rivers' mouths, in flocks of 2 to
  6, by day. Wild ones despawn far from players.
- **Life on the coast** ✅: it glides along the shore and soars in circles above it (the library's flight), lands on
  the sand, on rocks, posts and roofs, walks about, and **floats on water**, bobbing on the waves. It calls often (a
  cry, a laughing call), louder when it takes off. At night the flock rests on the shore.
- **A food thief** ✅, its signature: a player holding food in their main hand within 12 blocks of a wild seagull
  tempts it; now and then (about one chance in 4 a minute) it swoops down and **snatches one** of it (cooked fish
  and bread above all) and flies off with it. Hit it and it drops the food. It also steals food lying on the ground,
  and a fish a fishing player just caught. A sneaking player, or food in the off hand, is left alone.
- **Follows boats** ✅: it follows a player in a boat at sea for a while, hovering behind them, waiting for a fish.
- **Drops** ✅: 0 to 2 seagull feathers and 1 raw seagull (cooked when it burns), §5.

## 3. Befriending ✅

- **With fish** ✅: thrown raw cod or salmon (dropped near it, as the vulture's chicken), it eats; one chance in 4 it
  befriends the player who threw it (hearts).
- **Befriended** (the library's taming and ownership, §6.2): it follows its friend, flying above and landing near
  them, never steals from them, and stays or follows on right-click. Named with a name tag. It never despawns.
- **It fishes for its friend** ✅: only while its friend is **fishing** (their bobber in the water) and it is following
  them (not told to stay), about **once a minute** it flies out over the water and **plunge-dives** like a real gull: a
  steep dive with its wings folded back, a splash, a moment under, then it climbs out with a fish in its beak and
  **drops it at its friend's feet** (a raw cod, rarely a salmon, more rarely a tropical fish). It never misses.
- **Circles over a lucky spot** ✅: when its friend is in a boat or fishing, it flies ahead and **circles over a spot
  of water** within 16 blocks, calling: fishing there (the bobber within 4 blocks under it) is luckier, as one level
  of Luck of the Sea. It moves to a new spot every few minutes.

## 4. Bestiary page ✅

The biomes met in, and the facts: "Thief" (had food stolen by one), "Fish" (befriended with fish), "Catch" (brought a
fish), "Spot" (fished over its lucky spot), "Boat" (followed your boat).

## 5. Items ✅

- **Seagull feather** (`seagulls:feather`): white with a grey tip; used as a feather like the vulture's, in arrows, the
  brush and the book and quill. The recipes are the library's, through the tag `creature:feathers` (common/SPECS.md
  §3.8), which this mod adds its feather to.
- **Raw seagull** and **cooked seagull** (`seagulls:raw_seagull`, `seagulls:cooked_seagull`): ordinary meat like the
  vulture's, as a chicken's (their item templates: raw 2 hunger with a chance of Hunger, cooked 6), cooked in a furnace, a smoker or on a campfire.

## 6. Out of scope for v1

- Riding (too small), breeding, eggs and chicks, nests on cliffs, other gulls (black-headed gull, albatross).

## 7. Open questions

- ~~Q1: The food thief?~~ One chance in 4 a minute, one item, sneaking players left alone (§2).
- ~~Q2: Befriending?~~ Thrown raw fish, one chance in 4 (§3).
- ~~Q3: What a befriended one does?~~ It plunge-dives for fish and brings them back, and circles over a lucky fishing
  spot (§3).
- ~~Q4: Its drops?~~ Seagull feathers and seagull meat (§5).
