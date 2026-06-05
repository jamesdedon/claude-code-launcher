# Sprites & Animations

The launcher can host a small animation inside the prompt card. Animations are
**pluggable** and selected by a string in `config.toml`. There are two kinds:

- **Built-in** — compiled into the binary, selected by `name` (`little_guy`,
  `spinner`, `f1`, `none`). One vector character (`little_guy`) plus sprite
  sheets.
- **File-based** — a PNG sprite sheet plus a `.toml` manifest on disk, selected
  by `file`. This is the primary way to add new animations: drop two files in a
  folder, point the config at the manifest, no recompile.

This document specifies both, and the shared motion model underneath.

---

## 1. The `[animation]` config block

In `~/.config/claude-code-launcher/config.toml`. Use **either** `name` or
`file` (file wins if both are present). With **no `[animation]` block at all,
no animation runs.**

| Key          | Type                          | Default       | Meaning |
|--------------|-------------------------------|---------------|---------|
| `name`       | string                        | `"little_guy"`| A built-in: `little_guy`, `spinner`, `f1`, `none`/`off`. |
| `file`       | string (path)                 | —             | Path to a sprite manifest `.toml`. `~/` is expanded. Overrides `name`. |
| `cycle`      | string                        | per-animation | `once` \| `loop` \| `hold`. Overrides the animation's lifecycle. |
| `enter_from` | degrees \| edge \| `{dx,dy}`  | per-animation | Direction it enters from (see §5). |
| `exit_to`    | degrees \| edge \| `{dx,dy}`  | per-animation | Direction it exits to. |

Anything set here **overrides** the manifest, which overrides built-in defaults.

```toml
[animation]
file = "~/.config/claude-code-launcher/anims/f1.toml"
cycle = "once"          # override the manifest's lifecycle just here
```

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
| `cycle`      | string                        | `loop`               | Lifecycle: `once` \| `loop` \| `hold` (see §6). |
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

The launcher pops the animation in when the prompt opens and, for one-shots,
stops redrawing once finished (looping ones keep going).

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

For every motion field: **config `[animation]` block → manifest → built-in
default.** So you can ship a manifest with sensible defaults and still tweak
`cycle`/`enter_from`/`exit_to` per-machine from the config block.

---

## 9. Built-in animations

| `name`       | Kind   | Default cycle | Enter → Exit  | Notes |
|--------------|--------|---------------|---------------|-------|
| `little_guy` | vector | `once`        | bottom ↑      | Peeks over the bottom edge; unscaled (overflows + masks). |
| `spinner`    | sprite | `loop`        | right → right | Procedural demo sheet; `fit 0.8`, `min_card 96`. |
| `f1`         | sprite | `once`        | left → right  | 8-bit car; under-damped spring (fights for grip); `fit 0.85`, `min_card 96`. |
| `none`/`off` | —      | —             | —             | No animation. |

---

## 10. Authoring workflow

1. **Start from a seed.** Export the built-in sheets to real files to edit:
   ```sh
   cargo run --example export_sheet ~/.config/claude-code-launcher/anims
   ```
   This writes `f1.png` + `f1.toml` and `spinner.png` + `spinner.toml`.
2. **Edit the PNG** in any pixel editor (keep frame size + grid consistent), and
   tweak the manifest.
3. **Point the config at it:**
   ```toml
   [animation]
   file = "~/.config/claude-code-launcher/anims/f1.toml"
   ```
4. Restart the launcher to see it.

### Full annotated example manifest

```toml
# A side-view racer that drives in from the left and takes off right.
sheet      = "racecar.png"   # next to this manifest
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

## 11. Adding a built-in (Rust)

Sprites-as-files is the recommended path, but a bespoke animation can be added
in code:

- Implement `Content` (appearance: `natural_size` + `draw(cr, t)`), or reuse
  `SpriteContent` with a procedurally-generated sheet.
- Register it in `anim::build()` by wrapping it in `Staged::new(...)` with an
  `Approach` (rest + enter/exit `Dir`), a `Spring`, and chaining
  `.with_lifecycle(...)`, `.with_fit(...)`, `.with_min_card(...)`.

The motion layer (`Staged`) handles placement, direction, squash, fit, and the
lifecycle uniformly for sprite and vector content alike.
