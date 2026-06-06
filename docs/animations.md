# Sprites & Animations

The launcher can host a small animation inside the prompt card. Animations live
as **drop-in pack files** in `~/.config/claude-code-launcher/anims/` — one
`.toml` per animation. `config.toml` points at one with a single line. Drop a
new pack in the folder to add it, delete a file to remove it, edit a pack and
relaunch — no recompile.

A pack is **self-contained**: a `.toml` (motion + all timings) plus the PNG
**sheet** it points at. The built-in packs (`speed_racer`, `racer`, `f1`,
`little_guy`, `spinner`, `cherry_blossoms`) and their sheets are seeded into the
folder on first run; they're ordinary files you can edit, copy, or delete, with
nothing about them named in the engine.

This document specifies the pack format and the shared motion model underneath.

---

## 1. Selecting a pack — the `[animation]` block

In `~/.config/claude-code-launcher/config.toml`, one line points at a pack:

```toml
[animation]
name = "speed_racer"     # -> anims/speed_racer.toml
# file = "~/some/where/my_pack.toml"   # ...or an explicit path
```

| Key    | Type          | Meaning |
|--------|---------------|---------|
| `name` | string        | Pack filename stem in `anims/` (loads `<name>.toml`). |
| `file` | string (path) | Explicit path to a pack `.toml`. `~/` expands. Wins over `name`. |

With **no `[animation]` block, nothing runs.**

### 1.1 The two slots — `[spawn]` and `[submit]`

A pack has up to two sections, each optional; one file can fire **both**:

- **`[spawn]`** — plays when the launcher opens.
- **`[submit]`** — plays when you press Enter. The window stays open until this
  section finishes its exit, *then* closes (so the car drives off before the
  launcher vanishes) and only then spawns the terminal. A submit section is
  always forced to a **single run** (any `cycle` coerced to `once`) so the close
  can't hang.

```toml
# anims/speed_racer.toml
[spawn]
sheet        = "racer.png"   # a tall figure, panned over
frames       = 8
frame_width  = 88
frame_height = 240
pan          = { reveal = 2.6, hold = 0.7, exit = 1.2, slice = 72, focus = -8 }
rest         = [0.75, 0.5, 0, 0]
fit          = 0.92
min_card     = 120

[submit]
sheet        = "f1.png"
frames       = 8
cycle        = "once"
enter_from   = "left"
exit_to      = "right"
rest         = [0.25, 0.62, 0, 0]
fit          = 0.85
min_card     = 96
```

A **flat** manifest (fields at top level, no `[spawn]`/`[submit]`) is treated as
a single section routed by a `trigger = "spawn" | "submit"` field (default
spawn).

Each section draws a PNG `sheet` (see §2–§3); nothing about an animation is named
in code. The built-in default packs ship their sheets alongside their `.toml`,
all in `anims/`.

A section's motion, frame, and pan fields are specified in §3–§7.

---

## 2. The sprite sheet (PNG)

A single PNG containing every frame laid out in a grid, **left-to-right,
top-to-bottom**.

- **Transparency:** use an alpha channel (ARGB). Whatever isn't the character
  should be fully transparent.
- **Uniform cells:** every frame is the same `frame_width × frame_height`. With
  `columns` columns, frame `i` sits at column `i % columns`, row `i / columns`.
- **Single row is fine:** omit `columns` and it defaults to `frames` (one row).
- **Pixel art:** author small and set `pixelated = true` so frames are sampled
  nearest-neighbour (crisp pixels) when scaled by `fit`/`squash`. Without it,
  scaling smooths/blurs the pixels.

> Frame dimensions can be inferred from the sheet (`sheet_width / columns`,
> `sheet_height / rows`), so `frame_width`/`frame_height` are optional — but
> setting them explicitly avoids surprises with padded sheets.

---

## 3. The manifest (`.toml`) reference

The manifest lives next to its PNG; `sheet` is resolved **relative to the
manifest's directory** (absolute paths and `~/` also work).

### Sprite fields

| Field          | Type                    | Default            | Meaning |
|----------------|-------------------------|--------------------|---------|
| `sheet`        | string (path)           | **required**       | PNG sheet, relative to this manifest. |
| `frames`       | integer                 | **required**       | Total number of frames. |
| `columns`      | integer                 | `frames`           | Frames per row in the sheet. |
| `frame_width`  | number (px)             | `sheet_w/columns`  | Width of one frame. |
| `frame_height` | number (px)             | `sheet_h/rows`     | Height of one frame. |
| `fps`          | number                  | `12`               | Frame playback rate. |
| `anchor`       | `"center"`/`"bottom"`/`"top"` or `{x,y}` | `"center"` | The point **within a frame** that sits at the rest position and is the pivot for squash (see §4). |
| `play`         | string                  | `"loop"`           | Frame mode: `loop` \| `once` \| `pingpong` (see §6). |
| `pixelated`    | bool                    | `false`            | Nearest-neighbour sampling for crisp pixels. |

### Motion fields

All optional; these mirror the built-in motion vocabulary.

| Field        | Type                          | Default              | Meaning |
|--------------|-------------------------------|----------------------|---------|
| `cycle`      | string                        | `loop`               | Lifecycle: `once` \| `loop` \| `hold` (see §6). Ignored when `pan` is set. |
| `trigger`    | string                        | `spawn`              | `spawn` \| `submit`, only for a **flat** manifest (in a `[spawn]`/`[submit]` section the slot is implied). |
| `pan`        | `{ reveal, hold, exit, slice, focus }` | none      | A vertical camera pan up a tall figure (see §6.1); makes the section a single run timed to the pan. |
| `enter_from` | degrees \| edge \| `{dx,dy}`  | `left` (180°)        | Where it slides in from (see §5). |
| `exit_to`    | degrees \| edge \| `{dx,dy}`  | `right` (0°)         | Where it slinks out to. |
| `rest`       | `[nx, ny, dx, dy]`            | `[0.5, 0.5, 0, 0]`   | Resting position (see §4). |
| `fit`        | number (0–1)                  | none (unscaled)      | Scale content so its height ≈ `fit ×` card height (see §7). |
| `min_card`   | integer (px)                  | none                 | Minimum prompt-card height so the content has room (see §7). |
| `stiffness`  | number                        | `200`                | Spring stiffness (higher = snappier). |
| `damping`    | number                        | `16`                 | Spring damping (lower = bouncier / more overshoot). |
| `squash`     | number                        | `0.02`               | Squash-and-stretch amount, scaled by velocity. |

---

## 4. Coordinates, rest, and anchor

The drawing surface is the **card interior**, in device pixels — call it the
*stage* (`w × h`). Two anchors matter:

- **`rest` = `[nx, ny, dx, dy]`** — where the animation settles, in stage space:
  `x = nx·w + dx`, `y = ny·h + dy`. So `[1, 1, -58, 37]` reads as "bottom-right
  corner, 58px in from the right, 37px **below** the bottom edge" (the little
  guy, so only his head peeks over the masked edge). `[0.5, 0.5, 0, 0]` is dead
  centre.
- **frame `anchor` = `{x, y}`** (or a named position) — the point *inside a
  frame* that gets placed at `rest`, and the pivot around which squash happens.
  Named values resolve to: `center` = `(fw/2, fh/2)`, `bottom` = `(fw/2, fh)`,
  `top` = `(fw/2, 0)`. Use `bottom` for a character standing on the ground,
  `center` for a vehicle that flies through.

The card uses `overflow: hidden` with rounded corners, so anything outside the
card (e.g. a character resting partly below the bottom edge) is **masked** — the
basis of the "peek" effect.

---

## 5. Direction (`enter_from` / `exit_to`)

Direction is the heart of "move in any direction". It accepts three forms:

- **Degrees** — counter-clockwise, screen-y-down: `0` = right, `90` = up,
  `180` = left, `270` = down. The host computes a distance that carries the
  content fully off-stage along that angle.
  ```toml
  enter_from = 270   # rises up from below
  ```
- **Named edge** — sugar for common angles: `right` (0), `top`/`up` (90),
  `left` (180), `bottom`/`down` (270), plus the corners `top-right` (45),
  `top-left` (135), `bottom-left` (225), `bottom-right` (315).
  ```toml
  enter_from = "left"
  ```
- **Scalars** — an explicit pixel offset for precise or *partial* motion (e.g. a
  half-peek that doesn't fully leave):
  ```toml
  exit_to = { dx = -180, dy = 0 }
  ```

Enter and exit are **independent**: pop up from below and slink off to the side
if you like.

---

## 6. Lifecycle (`cycle`) vs. frame mode (`play`)

Two different time loops — don't confuse them:

- **`play`** controls how the **sprite frames** advance over time:
  - `loop` — frame `= floor(t·fps) mod frames`, forever.
  - `once` — advance to the last frame and hold.
  - `pingpong` — bounce 0→N→0 forever.
- **`cycle`** controls the **whole animation's lifecycle** (enter/exit):
  - `once` — enter, idle briefly (~1.3s), exit, then **done** (a single run).
  - `loop` — enter, idle (~2.5s), exit, wait (~1.0s), then repeat — forever.
  - `hold` — enter, then idle **until dismissed** externally (does not auto-exit).

A typical race car uses `play = "loop"` (wheels keep spinning) with
`cycle = "once"` (it drives through exactly once).

A `[spawn]` section pops in when the prompt opens; a `[submit]` section plays on
Enter (see §1.1) and is always forced to a single run. For one-shots the
launcher stops redrawing once finished (looping ones keep going).

### 6.1 Pan (`pan`) — a vertical camera move

A `pan` table turns a section into a vertical **camera pan** up a figure taller
than the card: a `slice`-tall window starts at the feet, pans up to `focus`,
holds, then continues out the top — leaving the card clear (the racer). Setting
`pan` makes the section a **single run** timed to the pan, so `cycle` is ignored
and the spring `enter_from`/`exit_to` should be zero (the pan is the motion).

| Field    | Unit    | Default | Meaning |
|----------|---------|---------|---------|
| `slice`  | frame px | `72`   | Height of the visible window. |
| `focus`  | frame px | `-8`   | Window-top to settle on (small negative frames the top with headroom). |
| `reveal` | seconds | `2.6`   | Pan from the feet up to `focus`. |
| `hold`   | seconds | `0.7`   | Hold on `focus`. |
| `exit`   | seconds | `1.2`   | Continue from `focus` up and out of frame. |

```toml
pan = { reveal = 2.6, hold = 0.7, exit = 1.2, slice = 72, focus = -8 }
```

Pan works on any sheet — it just clips a vertical window over the frame.

---

## 7. Fit-to-card and sizing

The empty prompt is short (~50px). Two knobs make a full-body sprite sit
properly at any prompt height:

- **`min_card`** — the animation asks the prompt to be at least this tall, so it
  has room. The card still grows further as you type.
- **`fit`** — scales the content so its natural height ≈ `fit ×` the card
  height. `fit = 0.85` leaves a little headroom; omit `fit` to draw at native
  size (the little guy does this, since he's *meant* to overflow and peek).

The spring also applies **squash-and-stretch along the travel axis** (amount =
`squash`, scaled by velocity), so motion reads with weight regardless of
direction.

---

## 8. Precedence

Within a section, each field is **manifest value → engine default** (a small
generic baseline; see `Defaults::sheet`). Whatever the section sets overrides it,
so each pack `.toml` fully specifies its animation. The `config.toml`
`[animation]` block doesn't override fields — it only **selects** the pack.

---

## 9. Seeded packs

These ship in `anims/` on first run (written only if missing, so your edits and
deletions stick). Each is an ordinary `.toml` + PNG sheet — nothing is named in
the engine.

| pack             | slots          | what it does |
|------------------|----------------|--------------|
| `speed_racer`    | spawn + submit | Driver pans into view on open, then the car launches off the line on submit. |
| `racer`          | spawn          | Just the driver pan. |
| `f1`             | submit         | Just the car launching on submit. |
| `little_guy`     | spawn          | The little guy peeking over the bottom edge. |
| `spinner`        | spawn          | A looping sprite drifting in. |
| `cherry_blossoms`| spawn          | A field of petals drifting top→bottom (seamless loop). |

The sheets (`racer.png`, `f1.png`, `spinner.png`, `little_guy.png`,
`blossoms.png`) are generated from code by `cargo run --example gen_assets` and
committed under `assets/anims/`; the binary embeds that folder and seeds it.

---

## 10. Authoring a pack

1. **Copy a seed.** Start from a seeded pack in `anims/`, e.g. `cp
   anims/speed_racer.toml anims/my_thing.toml` (and a sheet to point at).
2. **Edit it.** Point each section at a PNG `sheet` (see §2–§3 for the
   sheet/frame fields). Set motion, `pan`, `rest`, `fit`, `min_card`. Put a
   `[spawn]` and/or `[submit]` section in the one file.
3. **Select it:**
   ```toml
   [animation]
   name = "my_thing"     # -> anims/my_thing.toml
   ```
4. Relaunch — no rebuild.

### Full annotated pack

```toml
# my_thing.toml — a side-view racer that drives in on open.
[spawn]
sheet      = "racecar.png"   # your PNG, next to this file
frames     = 8
columns    = 8               # single row of 8
frame_width  = 152
frame_height = 72
fps        = 12              # wheel-spin speed
anchor     = { x = 76, y = 46 }   # ~centre of the frame
play       = "loop"          # frames cycle forever while on screen
pixelated  = true            # crisp 8-bit scaling
cycle      = "once"          # one drive-through, then gone
enter_from = "left"          # = 180 degrees
exit_to    = "right"         # = 0 degrees
rest       = [0.25, 0.62, 0.0, 0.0]   # settle in the left quarter
fit        = 0.85            # ~85% of card height
min_card   = 96              # grow the prompt to fit
stiffness  = 240.0           # snappy
damping    = 11.0            # under-damped: a little fight for grip
squash     = 0.02            # stretch with speed
```

---

## 11. Generating sheet art from code

The engine only plays PNG sheets — it never draws an animation itself. The
built-in default sheets happen to be *generated* from code as a convenience: the
generators live behind `cargo run --example gen_assets`, which renders each one
(`SpriteContent::f1_car`, `racer::racer_sheet`, `little_guy::little_guy_sheet`,
`blossoms::blossoms_sheet`, …) into `assets/anims/*.png`. To add or change one,
edit/add a generator, re-run `gen_assets`, and commit the PNG — or just author a
PNG by hand and point a pack at it. Either way the runtime stays generic.

The motion layer (`Staged`) handles placement, direction, squash, fit, and the
lifecycle uniformly; `SpriteContent` adds the optional vertical `pan`.
