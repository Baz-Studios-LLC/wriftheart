# THE TEN — shard dungeon bosses

Baz's directive (2026-07-17): *"the bosses before were horrible. We need to rethink the
ten dungeon bosses and really make them impressive and have unique boss mechanics.
They should each be done one at a time to really be unique. A beholder, a hydra,
stuff like that."*

So: the js BOSS_DEF port is **scrapped**. The js bosses were one template — a 2x mob
sprite with a crown, cycling six shared attacks (summon/volley/slam/charge/lob/web)
with phase speed-ups. None of that ships. Each of the ten shard dungeons gets a
BESPOKE boss: its own large art, its own signature mechanic, its own arena behavior —
authored one at a time, playtested by Baz before the next one starts.

What we KEEP from the js: the boss HP bar (name + phase-tick bar that reddens),
knockResist ~0.92, the reward flow (gilded chest + shard + warp rune via boss_loot),
always-a-potion drop, and the arena door-seal. Everything else is new.

## The roster (build order = progression order)

| # | land (tier) | dungeon | boss | signature mechanic |
|---|------------|---------|------|--------------------|
| 1 | grassland (0) | crypt | **THE BONE COLOSSUS** | A giant assembled skeleton. At each ⅓ HP it COLLAPSES into a bone pile and its skull flies free — small, fast, evasive, takes bonus damage. Hurt it before it reassembles; each rebuild loses an arm (armless = lunging bites + bigger novas, faster). |
| 2 | greenmaw (0) | vinewarren | **THE WARREN HYDRA** | Three vine-serpent heads from burrows. A severed head regrows TWO unless its stump is struck in time. The heart bulb is only vulnerable while every head is down at once. |
| 3 | honeyglade (0) | hivehollow | **THE HIVE QUEEN** | Brood combs on the walls hatch waves — she is armored while any comb lives. A ring of royal-guard drones orbits her with one rotating gap. Honey slicks slow the floor. |
| 4 | arctic (1) | frostcavern | **THE GLACIER MAW** | Burrows beneath the ice leaving a racing crack, erupts under you. The floor progressively freezes to slide-physics. Bait its eruption into its own ice pillars to stun and expose it. |
| 5 | swamp (1) | bog | **THE ALL-EYE** | The beholder. Five eyestalks, each with its own beam — pull-ray (the tongue rig), slow-ray, poison lob, summon gaze. Kill the stalks one by one; then the great eye opens, and its stare punishes MOVEMENT (freeze-tag gaze). |
| 6 | petalwood (2) | petalhall | **THE BRIAR QUEEN** | Bullet-hell petal spirals. Live thorn hedges grow and reshape the arena mid-fight. Invulnerable while blooming — smash her surfacing roots to interrupt the bloom. |
| 7 | mushroom (2) | fungal | **THE MYCELIUM THRONE** | The boss is the ROOM. A spore carpet claims tiles one by one; pustule nodes erupt and hatch. Cut the network by smashing connector nodes to expose the throne-heart. |
| 8 | burnt (3) | charhall | **THE ASH TITAN** | Molten armor plates break off one at a time — each lost plate changes its moveset and speeds it up. Fire trails ignite the floor (and the furniture). Meltdown sprint at the end. |
| 9 | chaos (4) | riftvault | **THE UNMAKER** | Steals the rules: mirrors your controls, teleports, spawns false clones (the real one has a tell), and tears void rifts that eat the arena's edges. |
| 10 | starhollow (5) | wriftvault | **THE HOLLOW STAR** | The lights go out. It is visible only by its own glow; constellation beams link star-points you must break; meteors fall on telegraphed tiles. Each phase snuffs more of the light. |

Not in the ten: THE WRIFTHEART (Black Castle finale — its own later milestone) and
THE CHOIRMASTER (Saltmaze, already bespoke-designed in the js Kingsplitter quest —
ports separately). Bonus-cave dungeons keep the elite stand-in for now.

Progress note (2026-07-17): ALL TEN BUILT in one session. 1-6 screenshot-verified in
staged fights; 7-10 clippy/test/smoke-verified (the black-capture episode was flickering
— re-shoot 7-10 when it lifts). Baz has playtested NONE — his first runs are the tuning
pass for the whole roster. Rigs built along the way, shared for future use: Slowed
(honey/slow-beams), Hexed (control mirror), Pulled (reels), DungeonLights rewrite
(darkness fights), arena furniture via the spawn blockers param.

## Framework (src/app/boss/)

- `mod.rs` — BossPlugin; `spawn_authored(theme_key, ...) -> bool` dispatch called from
  spawn_room_boss (false = theme not authored yet → elite stand-in fallback);
  `BossName` component; the shared boss HP bar (js drawDungeonHud: name + 168px bar,
  ⅓ ticks, reddens by phase).
- One file per boss (`bone_colossus.rs`, ...) — each self-contained like the mimic:
  own component, own tick, own death system. Bosses are NOT MobDefs.
- Bosses carry `DungeonBoss` so the arena seal + boss_loot + shard/rune flow in
  navigate() work unchanged.
- Reuse the toolkit: EBolt/WebBolt/ArcRock projectiles, spawn_burst, the Pulled reel
  rig, dungeon-dark lighting, destructibles, banners.

## Status

- [x] 1. THE BONE COLOSSUS — built (PORT.md milestone 22)
- [x] 2. THE WARREN HYDRA — built (PORT.md milestone 23): heart bulb + 5 burrows,
  head strike/spit AI, stump cauterize-or-regrow-double, all-heads-down heart window,
  cauterize wound (-3, floor 1hp — the open heart must land the kill)
- [x] 3. THE HIVE QUEEN — built: 4 wall combs (armored while any live, each hatches
  real wasp drones, cap 3), rotating royal-guard ring w/ one gap, dive-bombs,
  stinger fans, honey slicks on the shared Slowed rig; tempo rises per comb lost
- [x] 4. THE GLACIER MAW — built: burrow/crack-chase/erupt cycle (untouchable under
  the ice), 4 solid pillars — bait the burst beside one to shatter it ON the maw
  (210f stun, defense -2); surfaced slither+lunge windows; hoarfrost spreads per dive
- [x] 5. THE ALL-EYE — built: 5 role-tinted orbiting stalks (puller ray on the Pulled
  rig / bolter fans / leech summoner / slower beam on the Slowed rig / gazer whose
  stare punishes MOVEMENT), orb blind-shut until all are plucked; open phase adds
  novas, whole-orb gazes, and wound-triggered blinks
- [x] 6. THE BRIAR QUEEN — built: bloom-invulnerable rose-monarch, twin petal-spiral bullet hell, surfacing roots (smash = 300f wilt interrupt), smashable self-wilting thorn hedges reshape the arena
- [x] 7. THE MYCELIUM THRONE — built: 5 spore nodes creep a carpet tile-by-tile (Slowed + sporeling hatches underfoot), node erupt-rings; network dead = carpet recedes + the throne spits back
- [x] 8. THE ASH TITAN — built: 3 riding armor plates (head/chest/legs), each break = faster + new move (dash, then slam nova), burning wake fire-trail decals w/ contact damage; meltdown at 0 plates
- [x] 9. THE UNMAKER — built: HEX mirrors held controls (play.rs Hexed rig, 240f spells), blink-teleports, false selves at each third (glint tell; wounding the real scatters them), permanent void tears
- [x] 10. THE HOLLOW STAR — built: rewrites DungeonLights every tick (torches OUT — its glow + the hero's lantern + shard gleams are ALL the light), 4 orbiting shards chain damaging constellation beams, telegraphed meteor rings; bared = the dark closes in and it comes for you
- [ ] Polish pass: boss-name splash on arena seal, bespoke boss sfx keys, MOBS codex
  boss cards (js kept 'boss:<theme>' bestiary entries)
