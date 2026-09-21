# Cornix Keymap README Visualization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Generate a readable Cornix LP keymap diagram from `cornix/main.vil`, embed it in the repository README, and keep it synchronized automatically for up to five in-use layers.

**Architecture:** A dependency-free Python renderer owns the Cornix-specific Vial row ordering, physical key positions, label formatting, and SVG output. The README references the tracked SVG, while a GitHub Actions workflow tests and regenerates it whenever the Vial source or renderer changes; pushes to `master` commit a changed SVG and pull requests verify that it is current.

**Tech Stack:** Python 3 standard library, SVG, `unittest`, GitHub Actions

## Global Constraints

- Read mappings only from `cornix/main.vil`; do not duplicate the keymap in hand-maintained documentation.
- Render no more than five in-use layers and fail if a sixth in-use layer appears.
- Preserve Cornix LP's split layout, column stagger, thumb clusters, two encoders, and encoder presses.
- Keep generation deterministic and free of third-party runtime dependencies.

---

### Task 1: Cornix Vial-to-SVG renderer

**Files:**
- Create: `scripts/render_keymap.py`
- Create: `tests/test_render_keymap.py`

**Interfaces:**
- Consumes: Vial JSON objects with `layout[layer][row][column]` and `encoder_layout[layer][encoder][direction]`.
- Produces: `render_keymap(vil: dict) -> str`, `used_layer_indexes(vil: dict) -> list[int]`, and the CLI `python3 scripts/render_keymap.py INPUT.vil OUTPUT.svg`.

- [x] **Step 1: Write failing tests for layer selection and Cornix ordering**

Create fixtures containing the eight Cornix matrix rows, two encoder mappings, five used layers, and trailing Vial placeholder layers. Assert that placeholder-only layers are omitted, right-hand rows are mirrored, encoder presses come from row 2/5 column 6, and a sixth assigned layer raises `ValueError`.

- [x] **Step 2: Run tests to verify they fail**

Run: `python3 -m unittest discover -s tests -v`

Expected: FAIL because `scripts.render_keymap` does not exist.

- [x] **Step 3: Implement parsing and layout mapping**

Implement `used_layer_indexes`, `map_layer`, and readable keycode labels. Treat `KC_NO` and `KC_TRNS` as unassigned, ignore the two encoder-press matrix slots when detecting placeholder layers, and preserve the raw QMK keycode in each SVG key's `<title>` element.

- [x] **Step 4: Implement deterministic SVG rendering and CLI**

Draw five-or-fewer vertically stacked layer cards using Cornix-specific coordinates. Use mirrored right-hand rows, separate three-key thumb clusters, highlighted `MO(n)` keys, and cards for both encoders' counter-clockwise, clockwise, and press mappings. Write UTF-8 with a trailing newline and only replace the output after successful parsing/rendering.

- [x] **Step 5: Run the tests to verify they pass**

Run: `python3 -m unittest discover -s tests -v`

Expected: all renderer tests PASS.

### Task 2: Generated diagram, README, and automation

**Files:**
- Create: `docs/cornix-keymap.svg`
- Create: `README.md`
- Create: `.github/workflows/render-keymap.yml`

**Interfaces:**
- Consumes: `cornix/main.vil`, `scripts/render_keymap.py`, and `tests/test_render_keymap.py`.
- Produces: the README image `docs/cornix-keymap.svg` and a repeatable local/CI generation command.

- [x] **Step 1: Generate the initial SVG**

Run: `python3 scripts/render_keymap.py cornix/main.vil docs/cornix-keymap.svg`

Expected: the SVG contains Layer 0 through Layer 4 and no empty exported layers.

- [x] **Step 2: Add the README**

Document `cornix/main.vil` as the source of truth, embed `docs/cornix-keymap.svg`, show the one-line regeneration command, and explain that the workflow updates the diagram after relevant pushes to `master`.

- [x] **Step 3: Add GitHub Actions synchronization**

On pull requests, run the unit tests, regenerate the SVG, and fail on a diff. On pushes to `master` and manual dispatches, run the same checks and commit only a changed `docs/cornix-keymap.svg` using `github-actions[bot]`; exclude the generated SVG from path triggers so the bot commit does not loop.

- [x] **Step 4: Verify generated output and repository hygiene**

Run:

```bash
python3 -m unittest discover -s tests -v
python3 scripts/render_keymap.py cornix/main.vil /tmp/cornix-keymap.svg
cmp docs/cornix-keymap.svg /tmp/cornix-keymap.svg
python3 -m xml.etree.ElementTree docs/cornix-keymap.svg
git diff --check
```

Expected: tests PASS, `cmp` and XML parsing exit 0, and `git diff --check` reports nothing.

- [x] **Step 5: Inspect the rendered SVG visually**

Rasterize the SVG locally and confirm that all five layer cards, 48 physical keys, both encoders, and labels are legible without clipping.
