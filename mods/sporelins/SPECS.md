# Sporelins: specification

A mod of the **Horizon Creature** series for **Minecraft Horizon** (Minecraft Java 26.3): small, shy mushroom sprites of
dark forests and mushroom fields. They pass for a mushroom when you come near, puff spores when startled, and plant
mushrooms where they walk; befriended, one follows you home and tends a mushroom patch there.

**Requires** the series' library, Horizon Creature (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

Planned: the spec and the model, no code yet.

- **Two models**, one per look (pictures in docs/MODELS.md), with the same groups so the same code poses both:
  - the red cap (`assets/models/sporelin.bbmodel`): about two thirds of a block tall, a rounded red cap with white
    spots over beige gills (`head`), a pale stem for a body with two dark eyes and pink cheeks (`body`), two stubby
    arms and two short legs;
  - the brown cap (`sporelin_brown.bbmodel`): a wide, flat tan cap (9 texels across, as the brown mushroom's) on a
    thicker, taller stem, the same face.

  Hiding, it pulls its cap down over its stem (a pose of `head`).

Next: the code once Horizon's launcher builds the pinned revision again
(docs/HORIZON-ISSUES.md, #15764). The library's side comes with it: taming and ownership move from the vulture to the
library (common/SPECS.md §6.2).

## 1. Technical context

- **Mod id**: `sporelins` ✅. Entity `sporelins:sporelin`, spawn egg `sporelins:sporelin_spawn_egg`. No item of its own
  ✅ (§4).
- Uses from the library: model drawing and sounds (§3.1, §3.2), hitbox size (§3.4), monsters sparing it (§3.4), the
  creature guide's pages (§7), and **taming and ownership** (§6.2), which this mod brings into the library (rule of two:
  the vulture and the sporelin).
- It copies the wandering trader (goals only), with a box the size of its look's model ✅: 0.45 × 0.65 blocks for a
  red cap, 0.55 × 0.65 for a brown cap (set at spawn, the library's `resize`).
- **Two looks** ✅: a red cap or a brown cap, like the two mushrooms, rolled at spawn and saved on the entity. A model
  and a texture each (`entity/sporelin/red`, `entity/sporelin/brown`), the brown one wider and flatter (§0).

## 2. The sporelin ✅

- A small mushroom creature: a wide cap for a head, a pale stem for a body, two dark eyes, stubby arms and legs. It
  waddles. Ambient and harmless, 6 health.
- **Spawning** ✅: dark forests, mushroom fields, old growth taigas and swamps, where it is dark (light ≤ 7 on its
  block, as mushrooms want), 1 to 3 together; red caps and brown caps alike, red ones more common in dark forests and
  mushroom fields. Wild ones despawn far from players.
- **Shy** ✅: a player who comes within 6 blocks (not sneaking) makes it **hide**: it sits down and pulls its cap over
  its stem, and passes for a mushroom until the player is 10 blocks away or stands still a few seconds. A sneaking
  player can come close.
- **A spore puff when startled** ✅: hit, or a player sprinting at it, it bursts a small cloud of spores (particles,
  a soft "pff") and runs off. Harmless.
- **Plants mushrooms** ✅: about once every 5 minutes, it plants a mushroom of its cap's colour on a dark block next to
  it where a mushroom can grow, never more than 4 within 5 blocks of it.
- Monsters leave it alone (library §3.4).

## 3. Befriending ✅

- **With a mushroom of its colour** ✅: a hiding or calm sporelin takes a red (or brown) mushroom from a sneaking
  player's hand (right-click); one chance in 3 it befriends that player (hearts), otherwise it eats it (a puff).
- **Befriended** (the library's taming and ownership, §6.2): it follows its friend, waddling, and teleports to them
  when far, like a tamed wolf; right-click makes it stay or follow. Named with a name tag. It no longer hides from its
  friend and never despawns.
- **Tending a patch** ✅: told to stay, it plants and tends mushrooms around where it stays (8 blocks), in the dark:
  planting as in §2, and now and then it makes a mushroom there spread to a next block.
- **Mushroom stew** ✅: give it an empty bowl (right-click) and it hands back a **mushroom stew**, once per in-game day;
  asked again the same day, it shakes its cap and the bowl stays in your hand.
- **Glows at night** ✅: a befriended one's cap spots glow softly in the dark (a full-bright layer, `emissive`, and a
  faint visual light around it, as the fireflies' `visuals`), lighting nothing in the world: a small night companion.
  Wild ones don't glow.
- Fed a mushroom of its colour, a befriended one heals (+2).

## 4. Drops ✅

- When killed: 1 mushroom of its cap's colour ✅. No item of its own ✅.

## 5. Bestiary page ✅

The biomes met in, its looks (red and brown caps), and the facts: "Hides" (seen hiding), "Mushroom" (befriended with a
mushroom), "Patch" (a mushroom it planted), "Stew" (a bowl of stew it gave).

## 6. Out of scope for v1

- Breeding, riding, other mushrooms (crimson or warped fungi in the Nether), a jar for it (it is too big).

## 7. Open questions

- ~~Q1: Shy?~~ Yes: it hides as a mushroom, and its spore puff is harmless (§2).
- ~~Q2: Befriending?~~ With a mushroom of its colour, one chance in 3 (§3).
- ~~Q3: What a befriended one does?~~ Tends a mushroom patch, gives a mushroom stew once a day, glows softly at night (§3).
- ~~Q4: Its drop?~~ A mushroom of its cap's colour (§4).
- ~~Q5: Its looks?~~ Two: a red cap and a brown cap (§1).
