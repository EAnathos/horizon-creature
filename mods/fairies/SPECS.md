# Fairies: specification

A mod of the **Horizon Creature** series for **Minecraft Horizon** (Minecraft Java 26.3): tiny shy fairies of the
flowery woods and meadows, glowing softly at dusk. Befriended with a sweet treat, a fairy follows you, circling you
in sparkles, and lightens your step: a gentle speed boost.

**Requires** the series' library, Horizon Creature (`creature`, [common/SPECS.md](../../common/SPECS.md)).

> ✅ = decided · ❓ = proposal to confirm · numbers are starting values, to balance in game.

## 0. Status

Planned: the spec and the model, no code yet.

- **The model** (`assets/models/fairy.bbmodel`, pictures in docs/MODELS.md): our own design, a slender little violet
  fairy with a large head: big violet eyes, pink cheeks, dark violet shoulder-length hair with bangs and side locks, a
  small white flower pinned on one side. A little fairy dress: a fitted violet bodice with puffed lilac sleeves, a
  pale sash tied in a bow at the back, a layered skirt flaring out to a hem of lilac points, glinting with white
  sparkles. Thin arms and legs, one arm raised, one leg bent back as she hovers. Four butterfly wings spread from one
  point between her shoulder blades (a big upper pair, a smaller lower pair), pale lilac with a violet rim. Facing
  +z like every model of the series. Its parts (`body`, `head`, `arm_*`, `leg_*`, `hem`, `hem_*`, `wings`,
  `wing_upper_*`, `wing_lower_*`) are ready for the flutter of its wings and its hovering.

Next: the code once Horizon's launcher builds the pinned revision again (docs/HORIZON-ISSUES.md, #15764).

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

- **With glow berries**: a sneaking player holding glow berries within 3 blocks draws a wild fairy, which lands on
  their hand (hovers before them) and nibbles one; one chance in 3 it befriends them (hearts, a chime). Sweet berries
  work too, one chance in 6.
- **Befriended** (the library's taming and ownership): it follows its friend, **circling their head** and leaving a
  faint trail of sparkles; right-click to make it stay or follow. Named with a name tag. It never despawns. It flies
  to its friend if they get more than 24 blocks away.
- **The speed boost** ✅: while it follows its friend within 8 blocks, the friend has **Speed I**, renewed every few
  seconds (no particles, the icon shown). Several fairies don't stack: Speed I at most. A fairy told to stay gives
  nothing.
- **Fairy dust**: once a day a befriended fairy sheds a pinch of fairy dust at its friend's feet.
- **Night light**: at night a following fairy lights its friend's surroundings a little (light 8 around it).

## 4. Bestiary page ✅

The biomes met in, and the facts: "Friend" (befriended one), "Dust" (got fairy dust from one), "Shy" (one
fled from you), "Jar" (caught one in a glass jar).

## 5. Glass jar ✅

- **Caught in the glass jar** (library §4.1), the same one as butterflies, worms and fireflies: right-click a fairy
  with an empty jar. A wild one is only caught when it is close (a sneaking player); a befriended one only by its
  friend, and it stays befriended in the jar and once let out. Its name is kept.
- **Released**: a wild fairy let out on a flower patch settles there (the butterflies' territory, library §4.2); a
  befriended one follows its friend again.
- **A fairy lamp**: placed as a decorative jar (library §4.1, "display"), the fairy flutters inside and glows: light 10,
  brighter than a jar of fireflies.

## 6. Items ✅

- **Fairy dust** (`fairies:fairy_dust`): a glittering pinch. Brewed instead of sugar for a **Potion of Swiftness**
  (awkward potion + fairy dust), and thrown on a sapling or a crop (right-click) as bone meal, with sparkles.

## 7. Out of scope for v1

- Riding (too small), fairy rings and houses, other blessings (jump boost,
  night vision), fairies fighting for their friend, breeding.

## 8. Open questions

- ~~Q1: The name?~~ Fairies, `fairies` (§1).
- ~~Q2: Their look?~~ One violet fairy: big eyes, violet hair, a little fairy dress, butterfly wings (§2).
- ~~Q3: Wild?~~ Shy, fleeing runners; killing one drops nothing (§2).
- ~~Q4: Befriending?~~ Glow berries one chance in 3, sweet berries one in 6 (§3).
- ~~Q5: The speed boost?~~ Speed I within 8 blocks, not stacking (§3).
- ~~Q6: Its gifts?~~ Fairy dust once a day, a little light at night (§3, §6).
- ~~Q7: The glass jar?~~ Yes, a fairy lamp (§5).
