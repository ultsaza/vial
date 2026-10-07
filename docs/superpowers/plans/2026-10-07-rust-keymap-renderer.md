# Rust keymap renderer implementation plan

> Execute this plan inline using the existing checkout. Keep the user's README edits. The user has authorized committing and pushing the feature branch.

**Goal:** Replace the Python renderer with Rust and let GitHub Actions generate diagrams without a local generation step.

**Architecture:** A Cargo binary reads Vial JSON and writes the same self-contained SVG with an atomic file replacement. Black-box integration tests exercise the binary. PR jobs generate a preview artifact; master jobs generate and commit the SVG.

**Tech stack:** Rust 2024, serde_json, tempfile, GitHub Actions.

## Constraints

- Keep Cornix's 48 physical keys, mirrored right-hand rows, thumb positions, and encoder rotations/presses.
- Ignore layers containing only KC_NO, KC_TRNS, and -1 in the first six matrix slots.
- Reject more than five used layers and inputs with no used layers.
- Preserve raw keycodes, XML escaping, label formatting, SVG geometry, and deterministic output.
- Preserve existing output when input validation or rendering fails.
- Only master automation can write to the repository. PR checks must work without a committed SVG update, including fork PRs.
- No commit-hook installation is required. Keep the existing README title and image.

## Task 1: Characterize and port the renderer

Files: add Cargo.toml, Cargo.lock, scripts/render_keymap.rs, tests/render_keymap.rs, tests/fixtures/single-layer.vil, tests/fixtures/single-layer.svg; remove scripts/render_keymap.py and tests/test_render_keymap.py after verification.

- [x] Create a small single-layer input and record its SVG with the existing Python renderer.
- [x] Add a Cargo binary with an empty main and CLI integration tests. A test such as `assert_eq!(fs::read_to_string(output).unwrap(), include_str!("fixtures/single-layer.svg"))` must fail because the binary does not produce a diagram.
- [x] Verify that tests fail from the missing rendering behavior, then implement the Rust CLI with two path arguments, --help, JSON validation, mapping, label conversion, SVG construction, and temporary-file replacement.
- [x] Run `cargo test --locked`. Tests must cover unused/nonconsecutive layers, six-layer rejection, mirrored rows/thumbs, encoders, labels, escaping, invalid shapes/JSON, output creation/replacement, failure preservation, and CLI usage.
- [x] Compare Python and Rust output for cornix/main.vil byte-for-byte before removing Python.

## Task 2: Automate generation and document it

Files: update .github/workflows/render-keymap.yml, README.md, .gitignore.

- [x] Add Cargo/Rust source/test paths to workflow triggers and install Rust with rustfmt and clippy.
- [x] Run `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, and `cargo test --locked` in both jobs.
- [x] Render with `cargo run --locked -- cornix/main.vil docs/cornix-keymap.svg`. Upload the generated SVG in PR checks instead of rejecting an uncommitted generated diff.
- [x] Serialize master updates, check out the current master branch, and commit only docs/cornix-keymap.svg when changed. Keep PR permissions read-only and master contents:write.
- [x] Replace the README's manual Python steps with the automatic push/PR behavior and optional Cargo commands. Ignore /target/.
- [x] Run Cargo checks, repeat generation into two temporary paths, compare with the tracked SVG, parse the result as XML, validate workflow YAML with actionlint, and inspect git diff --check.

After these checks pass, commit and push `refactor/rust-keymap-renderer`. Integration into master is a separate step.

## Verification evidence

- Existing Python baseline: 6 tests passed. Rust tests first failed for missing rendering/validation, then all 14 passed.
- cargo fmt, cargo clippy with warnings denied, cargo test --locked, actionlint, and git diff --check passed.
- Actual cornix/main.vil output matched Python and the committed SVG byte-for-byte: 51,589 bytes, 5 layers, 240 keys, 10 encoders. Two independent Rust generations also matched.
- Ran the workflow run commands in an isolated temporary Git checkout with a local bare origin. An input-only change passed the PR commands; master commands generated and pushed a bot commit containing only the SVG; a repeat with no diagram difference created no commit.
- These checks verify local behavior; GitHub-hosted execution is a separate verification step after publication.
