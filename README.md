# STICK SNIPER — Operation Jailbreak

A 3D first-person stick-man sniper infiltration game that runs in the browser.
**Rust → WebAssembly**, rendered with **WebGL2**, HUD on Canvas2D, all sound synthesised with WebAudio
(no assets, no game-engine crates — just `wasm-bindgen`/`web-sys`).

## Play

```sh
./build.sh      # cargo build (wasm32) + wasm-bindgen → www/pkg
./serve.py      # http://localhost:8731 — no caching, and the page reloads itself when a new build lands
```

`serve.py` is a dev server: it sends `Cache-Control: no-store`, exposes `/__build`, and `main.js` polls that and
reloads the page after you rebuild (if you're mid-mission it waits until you pause). The title screen and pause
menu show the build time, so you can always tell which build you're running. Any other static server works too —
you just have to reload the tab yourself.

Needs: Rust + the `wasm32-unknown-unknown` target, and a `wasm-bindgen` CLI matching the crate (0.2.127).

## The mission

You start **low and far away** in the woods south of the valley. Being far away doesn't make you safe: the map is
full of things that block sight, and guards that hear a bullet crack past come looking.

1. **Work your way in** through the village: little houses with rooms, upstairs floors, flat-roof terraces, a church.
   **Lookouts** give you the high ground if you can take them (each guarded). They're the octagonal **bell tower** with a
   stair folded up inside it, the octagonal **hill lookout** house, the farm's **water tower** and **barn loft**, and the
   prison's own towers. Standing on one for the first time gives +4 sniper rounds.
2. **Shoot the padlocked latch on a prison gate.** The south gate or the west side gate. The latch pulses orange. Hit
   it and the gate swings free (the clang brings nearby guards to look).
3. **Shoot the latch on the cell block door.**
4. **Free Inmate 13** (`E`, north-west cell). Alarm! Charges blow the rear wall and the north fence; reinforcements arrive.
5. **Escape** to the extraction van.

You never *have* to kill anyone. Clearing an area (the village, the hill, the farm, the yard, the cell block) drops
**health packs** there as a reward.

## Controls

| | |
|-|-|
| `WASD` / `Shift` / `Space` | move / sprint (stands you up, drops aim) / jump |
| `C` | crouch — **toggle** (`Ctrl` = hold) |
| `LMB` | fire |
| `RMB` or `Q` | aim / scope — **toggle**: click once, it stays on until you click again |
| `Wheel` or `X` | scope zoom (5× / 10×) while scoped — otherwise the wheel switches weapon |
| `Shift` (scoped) | hold breath to steady the crosshair |
| `R`, `1`–`4`, `E` | reload, weapons, interact |
| `Esc` | pause — shows every command, your objective, and the stealth legend (`Enter` or click to resume) |

Weapons: M24 bolt sniper, AK-74, silenced MK23 (barely alerts anyone), SPAS-12 shotgun.

**The sniper wounds, a headshot kills.** A body shot does 70 — guards have 100 HP, marksmen 120, the Warden 320 — so
it never kills outright, but any headshot does 350. Bots that survive a hit know exactly where it came from and
turn on you at once.

### Stealth eye (top-left)

An eye shows whether you've been spotted: **NOT DETECTED** (closed) → **SUSPICIOUS** (half open, a ring fills as
someone notices you) → **SEARCHING** (they lost you, eyes darting) → **DETECTED** (wide open, alarm rays).
Beside it: your stance and a **visibility** meter. Crouching and standing still shrink how far bots can see you;
sprinting and loud shots (a couple of seconds of muzzle flash) make you easier to spot.

### The bots

- **They see what they're looking at.** A guard staring straight at you spots you sooner and from much further away
  than one glancing sideways. They sweep their gaze as they patrol. Marksmen see out to ~100 m when looking your way.
- **Foliage hides you.** Trees, bushes and hedges block their sight (but not bullets). Walls and buildings block both.
- **Near misses.** A shot that cracks past or smacks into something next to them makes them flinch, duck, spin
  toward where it came from, get behind something, then come looking. If you're standing in the open with a muzzle
  flash, they'll probably see you.
- **They fight from cover.** They find spots that hide them crouched but let them shoot when they stand. They duck,
  pop up for a burst, lean round the side, and move if you flank them. Guards with pistols won't charge a sniper they
  can't reach; marksmen posted in towers stay up there and fight from behind the parapet. The Warden just comes for you.
- **They find their way.** A navigation grid (built at load, ~60k standing spots) lets them route through doorways and
  up stairs, and it knows which gates are still locked.
- **Ragdolls.** When someone dies the live pose hands over to a Verlet ragdoll. It's kicked by the bullet, folds over
  walls and slides down steps, and the gun drops separately. Bodies stay where they fall, and other guards who find
  them raise the alarm.

## Code map (`src/`)

- `lib.rs`: `Game` (wasm API), player movement (slides along walls at any angle, camera glides over steps),
  weapons, latches, near-miss startles, pickups, the mission state machine
- `world.rs`: terrain heightfield; collision as **convex prisms** in a spatial grid, each flagged as stopping bullets,
  sight (foliage), feet (walkable) and/or movement. **Stairs are ramps** (a prism with a sloped top) for walking, with
  the drawn steps as separate bullet-stopping prisms, so climbing is perfectly smooth. Ray casts walk the grid (DDA).
- `builder.rs`: level toolkit. Walls at any angle with door and window openings, octagonal rings, floors with stair
  holes, straight flights, folded (switchback) stairs, octagonal lookouts, roofs, trees, hedges, fences
- `level.rs` (prison), `village.rs` (houses and church), `country.rs` (hill lookout, farm, woods)
- `nav.rs`: navigation grid plus A*, door-aware
- `enemy.rs`: AI (patrol, investigate, search, combat with cover and peeking; gaze-weighted sight; hearing; body discovery)
- `body.rs`: stick-figure pose (walk, crouch, aim, look around), drawing, and the ragdoll
- `render.rs`, `gfx.rs`: WebGL2 renderer. Baked soft sun shadows, hemisphere light, fog, wall AO in vertex colours,
  chain-link and scope mask in the shader
- `hud.rs`: minimal HUD (markers only for bots you can actually see; the radar shows bots you've recently spotted) ·
  `audio.rs`: procedural SFX · `weapons.rs`

`?nolock` on the URL skips pointer lock (used for automated testing); `window.game.dbg_*` are test hooks.
