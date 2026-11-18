# Fairies: specification

A mod of the **Sunrise Creatures** series for **Minecraft Horizon** (Minecraft Java 26.3): tiny shy fairies of the
flowery woods and meadows, glowing softly at dusk. Befriended with a sweet treat, a fairy follows you, circling you
in sparkles, and lightens your step: a gentle speed boost.

**Requires** the series' library, Sunrise Creatures (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

Built and tested in game (single player).

- **The fairy** (`fairies:fairy`, spawn egg): a copy of the wandering trader, box 0.4 × 0.8, 4 health, drawn
  full-bright with its own model (`assets/models/fairy.bbmodel`), its wings beating, its arms swaying, its hem
  turning, a soft violet light around it (seen only, lighting nothing), wider around one following its friend.
- **Wild** (§2): it flits between flowers with the library's small fliers' flight, day and night, resting in the rain.
  **Shy**: a player within 4 blocks who isn't sneaking (and holds no berries), or running within 8, scares it off.
  Spawning: flower forests, meadows, cherry groves and birch forests, 2 to 4 together, rarely, near players (the
  library's small fliers' spawning). Killed, it vanishes in sparkles and drops nothing.
- **Befriending** (§3): a player holding glow berries (or sweet berries) within 6 blocks draws it before their face;
  right-clicked with one, it eats it at once (one used up, not in creative) and befriends them one time in 3 (6 for
  sweet berries): hearts, a chime, a line on the action bar; otherwise a puff of smoke. Saved on the fairy (the
  library's `friend`).
- **Befriended**: it circles its friend's head, leaving faint sparkles; right-click makes it stay where it is or
  follow again. Its friend has **Speed I** while it follows within 8 blocks. Once a day it leaves a pinch of fairy dust
  at their feet. It comes to them at once past 24 blocks, never despawns, and its friend's blows can't hurt it (nor a
  player's arrow or trident: Horizon doesn't say whose).
- **Glass jar** (§5): caught as the other creatures (a befriended one only by its friend), still befriended once let
  out, its name kept; released on a flower it settles there; on display, a jar that glows (light 7, as fireflies).
- **Fairy dust** (§6): brews (awkward potion + fairy dust = Swiftness, `data/fairies/recipe/brewing/`).
- **Bestiary page**: the biomes met in, "Friend", "Catch".
- **Sounds**: its own, from CC0 freesound recordings (`assets/sounds.toml`): wings at take-off, a chime or a tiny laugh
  now and then, hurt, vanishing, the sparkle of a new friendship and of a gift. English and French subtitles.

Left: the Swiftness potion, with the next Horizon pin.

## 1. Technical context

- **Mod id**: `fairies` ✅. Entity `fairies:fairy` (one kind, one look), spawn egg
  `fairies:fairy_spawn_egg`, item `fairies:fairy_dust` ✅ (§6).
- Uses from the library: model drawing and sounds (§3.1, §3.2), hitbox size and monsters sparing it (§3.4), **small
  fliers** (§3.5: its wild flight, flitting between flowers), the creature guide's pages (§7), **taming and
  ownership** (§6.2, moving from the vulture with the Sporelins, the Seagulls and the Living Flowers), and the **glass
  jar and territories** (§4, which this mod switches on).
- It copies the wandering trader (goals only), a box of 0.4 × 0.8 blocks ✅. Drawn full-bright (it glows). Its own
  sounds (a soft chime, a tinkling laugh, wing flutter) from freesound, CC0, cut by `cargo creature sounds`.

## 2. The fairy ✅

- **Its look** (our own design): a slender little figure about three quarters of a block tall, a large head with big
  eyes, violet hair with a little flower, a violet fairy dress with puffed sleeves and a pointed hem, two pairs of
  butterfly wings spread from its back, a soft violet glow around it. 4 health.
- **One fairy, violet**: a single kind, no other colours.
- **Spawning**: in flower forests, meadows, cherry groves and birch forests, in little groups of 2 to 4, from dusk
  to dawn and on clear days near flowers. Wild ones despawn far from players.
- **Wild**: it flits from flower to flower (the library's small fliers), hovers, laughs. **Shy**: it flies away from
  a player who runs or comes close without sneaking, and from monsters. A sneaking player can come near.
- **Never hurt by its friends**: a befriended fairy can't be hurt by its friend's blows (no accidents).
- **Killing a wild one**: it vanishes in a puff of sparkles and drops nothing (fairies are not loot).

## 3. Befriending ✅

- **With glow berries**: a player holding glow berries within 6 blocks draws a wild fairy, which hovers before them;
  right-clicked with one, it eats it at once; one chance in 3 it befriends them (hearts, a chime). Sweet berries work
  too, one chance in 6.
- **Befriended** (the library's friends, common/SPECS.md §6.2): it follows its friend, **circling their head** and leaving a
  faint trail of sparkles; right-click to make it stay or follow. Named with a name tag. It never despawns. It flies
  to its friend if they get more than 24 blocks away.
- **The speed boost** ✅: while it follows its friend within 8 blocks, the friend has **Speed I**, renewed every few
  seconds (no particles, the icon shown). Several fairies don't stack: Speed I at most. A fairy told to stay gives
  nothing.
- **Fairy dust**: once a day a befriended fairy sheds a pinch of fairy dust at its friend's feet.
- **Night light**: at night a following fairy lights its friend's surroundings a little (light 8 around it).

## 4. Bestiary page ✅

The biomes met in, and the facts: "Friend" (befriended one), "Jar" (caught one in a glass jar).

## 5. Glass jar ✅

- **Caught in the glass jar** (library §4.1), the same one as butterflies, worms and fireflies: right-click a fairy
  with an empty jar, as the other creatures; a befriended one only by its friend, and it stays befriended in the jar
  and once let out. Its name is kept (shown under the jar's).
- **Released**: a wild fairy let out on a flower patch settles there (the butterflies' territory, library §4.2); a
  befriended one follows its friend again.
- **A fairy lamp**: placed as a decorative jar (library §4.1, "display"), the fairy flutters inside and glows: light 7, as a
  jar of fireflies.

## 6. Items ✅

- **Fairy dust** (`fairies:fairy_dust`): a glittering pinch. Brewed instead of sugar for a **Potion of Swiftness**
  (awkward potion + fairy dust). It doesn't make crops grow.

## 7. Out of scope for v1

- Riding (too small), fairy rings and houses, other blessings (jump boost,
  night vision), fairies fighting for their friend, breeding.

## 8. Testing in game

Survival and creative, a flower forest or meadow; `cargo creature install fairies` first.

- Wild ones appear near you now and then; walking up scares them off; sneaking, they stay.
- Holding glow berries: one hovers before your face; right-click feeds it at once; one in 3 befriends (hearts, a
  line on the action bar).
- Befriended: it circles your head, Speed I, fairy dust at your feet once a day, back to you past 24 blocks, unhurt by
  your sword; right-click: it stays, again: it follows; it survives a reload.
- In a jar (no sneaking): "Befriended" and its name on the jar, still yours once released.
- Its sounds: wings, chimes and laugh, hurt, vanishing, the sparkle on befriending.

## 9. Open questions

- ~~Q1: The name?~~ Fairies, `fairies` (§1).
- ~~Q2: Their look?~~ One violet fairy: big eyes, violet hair, a little fairy dress, butterfly wings (§2).
- ~~Q3: Wild?~~ Shy, fleeing runners; killing one drops nothing (§2).
- ~~Q4: Befriending?~~ Glow berries one chance in 3, sweet berries one in 6 (§3).
- ~~Q5: The speed boost?~~ Speed I within 8 blocks, not stacking (§3).
- ~~Q6: Its gifts?~~ Fairy dust once a day, a little light at night (§3, §6).
- ~~Q7: The glass jar?~~ Yes, a fairy lamp (§5).
