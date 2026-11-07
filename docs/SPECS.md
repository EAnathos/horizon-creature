# Vautour — spécifications (v0.2)

Mod natif Rust pour **Minecraft Horizon** (réimplémentation Rust de Minecraft Java 26.3).
Ajoute un vautour sauvage, qu'on apprivoise en lui lançant des poulets crus, qu'on monte, qu'on équipe de coffres,
qui attrape des créatures dans ses serres et qui se reproduit.

> ✅ = décidé · ❓ = proposition à valider · les chiffres sont des valeurs de départ à équilibrer en jeu.

## 0. Avancement (au 2026-11-07)

### Fait

- **Phase 0 réussie** (§1.2) : entité `vulture:vulture` (copie du marchand ambulant, ses goals remplacés), dessinée par le
  mod, montée et pilotée en vol. Testé en jeu, en solo.
- **Monter** : clic droit sur le vautour (sans cheats) ou `/ride` (cheats). Accroupi pour descendre ❓ : à confirmer avec
  une vraie touche après une montée par clic droit.
- **Vol monté**, sans endurance pour l'instant : ZQSD selon le regard, Espace monte, rien = plané, Ctrl = piqué.
  Pas de dégâts de chute pour le vautour.
- **Modèle et texture du vautour fauve** : `assets/models/vulture.bbmodel` (Blockbench, 17 cubes : tête, bec crochu, cou,
  collerette, corps, ailes en 2 segments, queue, pattes et serres), texture 64×64, converti en Rust par
  `tools/bbmodel_to_rust.py`. Animations : ailes repliées au sol, battement en vol, tête qui suit le regard, marche,
  pattes rentrées en vol.
- **Cœurs de la monture** : seulement ceux qu'il a, sans cœurs vides.
- **Sons** (§7) : le marchand est muet ; cri (sifflement, 3 variantes + cri rare), blessé, mort, battements d'ailes,
  décollage et manger (ces deux-là pas encore joués par le code), tirés de sons CC0 de freesound par `tools/make_sounds.py` (sources listées dans le script).
  Sous-titres FR + EN. Blessure et mort : codées, pas encore testées en jeu.
- **Œuf d'apparition** `vulture:vulture_spawn_egg` (onglet créatif « Vautour »).
- **Outils** : `tools/install-dev.sh` construit et installe le mod dans l'instance `vulture-dev`.

### Reste à faire

- §2 Statistiques variables (vie, vitesse, endurance) et leur hérédité.
- §3 Les 4 autres variantes (moine, percnoptère, gypaète, royal) : textures et tirage à l'apparition.
- §4 Vautour sauvage : hitbox (~1,2 × 1,0 ; il garde pour l'instant celle du marchand), IA (cercles en vol, se poser,
  charognard), neutre et riposte, apparition naturelle par biome, drops (plumes, viande) et leurs items.
- §5 Apprivoisement au poulet cru lancé, propriétaire.
- §6 Vautour apprivoisé : assis/suivre, défense du propriétaire, soin ; monter réservé au propriétaire avec harnais.
- §6.1 Harnais (item, recette, slot, visible sur le modèle).
- §6.2 Endurance : jauge à la place de la barre d'XP, coûts et recharge ; marche au sol et décollage.
- §6.3 Coffres (2 × 15 slots), écran du vautour.
- §6.4 Serres : touche, attraper et lâcher.
- §7 Petit (modèle et texture), harnais et coffres sur le modèle, animations picorer/manger, assis, attaque, serres.
- §8 Reproduction.
- Multijoueur (LAN) : non testé ; pour un invité, c'est son client qui déplace la monture.


---

## 1. Contexte technique

- **Format** : mod natif Horizon = un mod de contenu (items, recettes, loot, textures, sons, type d'entité en JSON)
  + des modules Rust injectés dans Horizon (`horizon-mixin.toml`, hooks `native_*`). Outil : `cargo horizon-mod`.
- **Toolchain** : Rust 1.99.1, épinglé par le launcher et déjà installé.
- **Cible** : jeu 26.3, révision Horizon épinglée au début du développement.
- **Id du mod** : `vulture`. Entité `vulture:vulture`, items `vulture:harness`, `vulture:feather`,
  `vulture:raw_vulture`, `vulture:cooked_vulture`.
- **Multijoueur** ✅ : v1 = solo + LAN. Le serveur dédié vient en v2, mais la logique serveur est séparée du rendu dès
  le départ.

### 1.1 Ce que l'API Horizon permet aujourd'hui (lu dans `docs/mods.md` et `docs/native-mods.md` d'Horizon)

| Besoin | Disponible | Remarque |
|---|---|---|
| Nouveau type d'entité vivante | ✅ testé en jeu | Uniquement en copiant un **zombie, un villageois ou un marchand ambulant**. `vulture:vulture` (marchand ambulant) marche : `/summon`, œuf, sauvegarde. |
| IA personnalisée | ✅ | `register_goals` : on vide les goals du modèle de base (marchand ambulant) et on met les nôtres |
| Modèle et texture personnalisés | ✅ testé en jeu | `native_mob_submit` : on dessine nos cubes (`native_draw_cube`, pile de transformations, pivots) avec des textures `vulture:entity/...` |
| Données sauvegardées par entité | ✅ | Attachments serveur sur `Holder::Entity(id)` (propriétaire, stats, variante, coffres…) |
| Clic droit sur l'entité | ✅ | `native_interact_entity` |
| Événement d'apprivoisement | ✅ | `native_entity_tame` (vanilla seulement, à vérifier) |
| Items au sol proches | ✅ | API 42 (position, stack, délai de ramassage) |
| Sons du mod | ✅ événement joué en jeu (fichiers `.ogg` pas encore écoutés) | `assets/vulture/sounds.json` + `.ogg`, joués par `Host::play_sound` (Horizon préfixe d'office les sons des mobs par `minecraft:`) |
| Écran d'inventaire | ✅ | Types de menu du mod (1 à 256 slots) |
| Apparition naturelle | ✅ | `add_spawns` par biome |
| **Monter une entité du mod et la piloter en vol** | ✅ phase 0 (2026-11-07) | `/ride` accepte tout mob ; Horizon copie les touches du cavalier dans `Mob.rider` (avancer, côté, saut, sprint), qu'un goal lit pour régler la vitesse. Champs internes : révision d'Horizon épinglée. Clic droit → monter : par une injection mixin qui appelle `start_riding` (`run_command` est refusé pendant le tour d'un joueur). Sans cheats. |
| Porter une entité (serres) | ❓ | Probablement en faisant d'une créature le passager d'une autre, ou en la téléportant chaque tick. À vérifier. |

### 1.2 Phase 0 — prototype technique, avant tout le reste

1. Déclarer `vulture:vulture` (copie du marchand ambulant), le faire apparaître avec `/summon` et le dessiner avec un cube.
2. Faire monter un joueur dessus et lire ses touches (ZQSD, Espace, Ctrl) côté serveur.
3. Le faire voler en contrôlant directement sa vitesse.

Si l'étape 2 ou 3 n'est pas possible, trois options :
- **A** : proposer à Horizon une contribution (dépôt open source) qui ajoute un modèle de base de type monture (cheval, âne, happy ghast) ;
- **B** : faire apparaître un âne vanilla, qui sait déjà être monté, porter des coffres, varier ses stats et se reproduire, et le redessiner en vautour ;
- **C** : coder la monture nous-mêmes via des hooks d'entrée joueur et de position.

---

## 2. Statistiques variables ✅ (comme les chevaux)

Chaque vautour tire ses stats à sa naissance ou à son apparition :

| Stat | Plage | Effet |
|---|---|---|
| Points de vie max | 16 – 30 | |
| Vitesse de vol | 0,6 – 1,2 × la vitesse de base | Plané et piqué |
| Endurance max | 70 – 130 | Durée de vol battu (§6.2) |

- **Reproduction** : comme les chevaux vanilla, chaque stat du petit est la moyenne des deux parents et d'une valeur
  tirée au hasard. Les bonnes lignées s'améliorent avec le temps sans dépasser les plafonds.
- Les stats ne sont pas affichées en jeu, comme en vanilla ❓. On pourrait plus tard les montrer avec un item ou dans l'écran du vautour.

## 3. Variantes de skin ✅

Inspirées d'espèces réelles, chacune avec sa propre texture :

| Variante | Inspiration | Apparition |
|---|---|---|
| Fauve | Vautour fauve (beige, collerette blanche) | Commune, tous les biomes |
| Moine | Vautour moine (brun très sombre) | Commune, badlands |
| Percnoptère | Percnoptère d'Égypte (blanc, face jaune) | Commune, désert |
| Gypaète | Gypaète barbu (poitrail orangé, « barbe » noire) | Peu commune, savane plateau / montagnes proches |
| Royal | Sarcoramphe roi (tête multicolore) | Rare (~3 %), partout |

- Un petit prend la variante d'un de ses parents (50/50), avec ~5 % de chances de mutation vers une variante au hasard ❓.
- Des marques secondaires (taches, ailes plus sombres) pourraient multiplier les combinaisons comme pour les chevaux — **plus tard** ✅ (v1 = les 5 variantes).

## 4. Vautour sauvage

| Propriété | Valeur |
|---|---|
| Taille (hitbox) | ~1,2 × 1,0 bloc ; envergure visuelle ~2,5 blocs en vol |
| Comportement ✅ | **Neutre et riposte** : attaque qui le frappe (3 dégâts), et les vautours proches s'en mêlent, comme les loups/lamas |
| Biomes ✅ | Désert, badlands, savane — groupes de 1–3 |
| Vol sauvage | Cercles lents en altitude, se pose sur des points hauts |
| Charognard | Attiré par la viande au sol et les mobs morts récemment (~16 blocs) |
| Drops ✅ | 0–2 **plumes de vautour** + 1–2 **viandes de vautour crues** (cuisinables, viande normale ✅) |

## 5. Apprivoisement ✅

1. Le joueur **lance** un poulet cru (touche Q) près d'un vautour sauvage. Le donner à la main ne marche pas.
2. Le vautour repère l'item (~16 blocs), se pose dessus et le mange (animation + particules).
3. **1 chance sur 4** par poulet : cœurs si c'est réussi, fumée sinon.
4. Le propriétaire est le joueur qui a lancé le poulet.
5. Les autres viandes crues sont mangées sans effet ; le poulet cuit est ignoré.

## 6. Vautour apprivoisé ✅ (comme le loup)

- **Assis / suivre** : en mode suivre, il vole au-dessus du propriétaire et se téléporte s'il est trop loin.
- Défend son propriétaire et attaque ce que le propriétaire frappe, comme le loup.
- Nom avec étiquette. Pas de dégâts de chute. Soin : viande crue (+4 PV).
- **Contrôles** ✅ :
  - clic droit = monter s'il a un harnais, sinon assis/suivre ;
  - accroupi + clic droit = écran du vautour (contient aussi un bouton assis/suivre) ;
  - clic droit avec de la chair putréfiée = mode reproduction (§8).

### 6.1 Harnais ✅

- Item **Harnais de vautour**, non empilable, recette : 5 cuir + 2 ficelle + 1 lingot de fer (en U, le fer au centre) ✅.
- Nécessaire pour le monter **et** pour porter des coffres ✅. Se pose dans un slot de l'écran du vautour, et se voit sur le modèle.

### 6.2 Vol avec endurance ✅

| Touche | Action |
|---|---|
| ZQSD | Direction selon le regard |
| Espace | Battre des ailes : prend de l'altitude, consomme de l'endurance |
| (rien) | Plané : descente lente vers l'avant |
| Ctrl gauche | Piqué : descente rapide, gain de vitesse |
| Accroupi | Descendre, **même en plein vol** ✅ (dégâts de chute normaux) |
| Touche « serres » ❓ (par ex. R) | Attraper / lâcher (§6.4) |

- Jauge d'endurance à la place de la barre d'XP quand on est monté, comme la jauge de saut du cheval.
- Battre des ailes : −20/s · Plané : +5/s · Au sol : +25/s.
- À 0, il ne peut plus battre des ailes avant d'être remonté à 30 % : il plane et redescend.
- ✅ Les coffres **ne pèsent pas** sur l'endurance. ✅ **Pas de courants ascendants.**
- Au sol : marche lente, décollage avec Espace.

### 6.3 Coffres ✅

- Jusqu'à **2 coffres**, un de chaque côté, visibles sur le modèle. On les ajoute par clic droit avec un coffre (harnais obligatoire).
- **15 slots par coffre**, donc 30 au total.
- **Écran du vautour** : slot harnais, boutons « retirer coffre gauche / droit » ✅ (le contenu tombe au sol), grille de 15 ou 30 slots, bouton assis/suivre.
- À sa mort, il lâche ses coffres, leur contenu et son harnais.

### 6.4 Serres ✅

- En vol, monté, avec la touche serres : attrape la créature juste en dessous (~3 blocs).
- Peut attraper :
  - les **mobs assez petits** (hitbox ≤ 1 bloc de large et ≤ 1 bloc de haut ✅ : poulet, lapin, cochon, mouton, renard, chat, loup, bébés…) ;
  - les **alliés** ✅ : un joueur accroupi sous le vautour (s'accroupir vaut accord), ou un animal apprivoisé.
- Porter quelqu'un **coûte de l'endurance** ✅ : les battements d'ailes coûtent +50 % ❓ et le plané ne recharge plus.
- Il lâche sur la touche, à la mort du vautour ou quand le cavalier descend. La chute fait des dégâts normaux.
- Un vautour sauvage ou non monté n'attrape rien ❓.

## 7. Contenu graphique et audio

- Le **modèle est dessiné par code** (`native_mob_submit`) à partir de cubes. La source est un fichier `.bbmodel` (Blockbench), converti par un script du dépôt en données Rust :
  on édite en visuel, le jeu reçoit des cubes, pivots et UV.
- Parties animées : tête, cou, corps, 2 ailes en 2 segments (bras + pointe), queue, 2 pattes avec serres, harnais, 2 coffres.
- Animations par code : battement, plané, piqué, marche, picorer/manger, assis, attaque, serres fermées.
- Textures 64×64 : une par variante + petit + harnais, peintes dans Blockbench.
- Sons : cri, battement d'ailes, manger, blessé, mort. Sons CC0 (freesound.org, licence vérifiée ; le CC0 n'oblige pas à créditer, les sources sont listées dans `tools/make_sounds.py`) retouchés.
- Œuf d'apparition, textures des items, traductions FR + EN.

## 8. Reproduction ✅ (v1)

- Deux vautours apprivoisés nourris avec de la **chair putréfiée** ✅ (des charognards…) passent en mode reproduction.
- Le petit naît apprivoisé, avec le même propriétaire, des stats (§2) et une variante (§3) héritées.
- Le petit est plus petit, avec une grosse tête et le crâne duveteux. Il devient adulte en 20 min (accéléré en le nourrissant), et on ne peut le monter qu'adulte.
- Délai de 5 min avant que les parents puissent se reproduire de nouveau.

## 9. Hors scope v1

- Serveur dédié (v2)
- Armure de vautour
- Afficher les stats en jeu
- Configuration utilisateur

## 10. Questions ouvertes restantes

- ~~Q26 — Si la phase 0 montre qu'on ne peut pas monter une entité du mod : A, B ou C (§1.2) ?~~ Sans objet : la phase 0 a
  réussi sans aucune des trois options (2026-11-07).
- Surcoût d'endurance quand il porte quelqu'un : +50 % ?
