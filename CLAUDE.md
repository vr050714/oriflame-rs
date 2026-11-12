# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Oriflamme (base card game, 3 to 5 players) for one human against computer players. Rust workspace, Bevy 0.20 UI, Windows is the development platform.

## Commands

```
cargo run                                   # play (debug build, console stays open for logs)
cargo build --release                       # size-optimised exe, no console window
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
cargo test -p oriflamme-core                # rules tests, fast
cargo test -p oriflamme-core archer         # tests whose name contains "archer"
cargo test -p oriflamme-ai --release -- --nocapture   # bot strength tests; slow, always use --release
cargo run -p oriflamme-art                  # regenerate assets/ from art/ after editing an SVG
cargo clippy -p oriflamme-rs --target wasm32-unknown-unknown -- -D warnings   # check the web build
sh web/build.sh                             # web build into dist/ (needs wasm-bindgen and wasm-opt on PATH)
python -m http.server -d dist               # serve dist/ locally
```

### Web build

- `.github/workflows/pages.yml` runs `web/build.sh` on every push to `main` and deploys `dist/` to GitHub Pages.
- `web/index.html` is the page; `web/build.sh` runs `wasm-bindgen --target web`, then `wasm-opt -Oz`, then copies the page and `assets/icon.png` into `dist/`.
- The `wasm-bindgen` CLI must have the same version as the `wasm-bindgen` crate in `Cargo.lock`. CI reads the version from `Cargo.lock`; locally, `cargo install wasm-bindgen-cli --version <that version>`. Locally `wasm-opt` comes from `cargo install wasm-opt`.
- The web build uses the `web` cargo profile (release without `strip`, because wasm-bindgen needs the `target_features` section; `wasm-opt` strips afterwards).
- Size: about 17 MB raw, 5.3 MB gzipped. Most of it is Bevy ECS code plus reflection, serde and shader compiling (naga, wesl), which Bevy always includes. The Bevy feature list is explicit on purpose: the `default_app`/`ui_api` bundles add animation, gizmos, HDR and `reflect_auto_register`. Measure with `twiggy top` on `target/wasm32-unknown-unknown/web/oriflamme-rs.wasm` before adding features.
- Native-only and web-only Bevy features live in the target-specific `[target.'cfg(...)'.dependencies]` sections of the root `Cargo.toml`. The web build uses WebGL2, has no threads, and compiles out logging below `warn`.
- Use `web_time` instead of `std::time::SystemTime`/`Instant`: the std versions panic in the browser.
- In the browser, `AsyncComputeTaskPool` runs on the main thread, so a bot's search freezes the page until it finishes.

- A cold Bevy build takes 10 to 25 minutes and a lot of memory on this machine. Use `-j 4` if memory is short. Changing Bevy features or adding a dependency that Bevy shares (for example `winit`) triggers a full rebuild.
- `cargo build --release` fails to overwrite the exe while the game is running. Close the game first.
- The root package has no tests; `cargo test` at the root only matters for the two library crates.

## Lints: no panics, no unsafe

`[workspace.lints]` in the root `Cargo.toml` denies `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `unreachable`, `indexing_slicing`, `arithmetic_side_effects`, `panic_in_result_fn`, and forbids `unsafe_code`. Every crate opts in with `[lints] workspace = true`. In practice:

- Integer arithmetic uses `saturating_*` / `checked_*` / `wrapping_*`. Float arithmetic is not flagged.
- Collections are read with `.get()`, `.first()`, iterators; never `v[i]`.
- Fallible paths return `Result` or `Option`.
- `clippy.toml` relaxes unwrap, expect, panic and indexing inside `#[test]` code only. Integration test files (`crates/*/tests/*.rs`) need their own `#![allow(...)]` at the top, because helper functions there are not `#[test]` items.

## Architecture

Four packages. Dependencies point one way: app -> ai -> core. `oriflamme-art` is a standalone tool.

### `crates/oriflamme-core`: rules engine

Pure state machine, no Bevy, no dependencies, no I/O.

- `GameState::legal_actions()` lists what the current player may do; `GameState::apply(action)` is the only way to change state. `apply` first checks the action is in `legal_actions()`, so illegal input returns `RuleError` and leaves the state unchanged.
- `Phase` drives everything: `Placement { player, placed }`, `Resolution { card, decision }`, `GameOver`. `Decision` is either `RevealOrNot` or `Ability(kind)` (a target must be chosen). After each action, `advance()` runs every following card that needs no choice (face-up Heir, Lord, and so on) until some player must decide or the round ends. One `apply` call can therefore produce many events and even start the next round.
- The queue is `Vec<Stack>`; a `Stack` is `Vec<QueueCard>` with the top card last. Only top cards are active. `Card { owner, kind }` is unique (one card of each kind per player), so cards are tracked by value, not by index; positions shift when cards are removed.
- `resolved: Vec<Card>` records which cards were handled this resolution phase. A card uncovered mid-phase is added to it, so it waits until the next round.
- Randomness comes from the crate's own `Rng64` (SplitMix64), passed in by the caller. There is no `rand` dependency; games replay from a seed.
- `view.rs`: `GameState::view_for(player)` gives a `PlayerView` with opponents' hands, removed cards and face-down cards hidden. `PlayerView::determinize(rng)` builds a full `GameState` consistent with a view by filling hidden cards at random. A test asserts `determinize(view).view_for(player) == view`.
- `apply_traced(action)` returns `Vec<Step>`: each `Event` with a `Snapshot` (queue plus influence) before and after it. It relies on one rule: every `self.emit(trace, event)` call must come right after that event's own change to the table and nothing else. Keep to that when adding events. Positions inside events refer to the "before" table for cards that leave or move, and to the "after" table otherwise (documented on `Event`).
- Rule choices where the rulebook is silent are listed in the crate doc comment at the top of `lib.rs`. The ability text was checked against the published rulebook; do not change card behaviour from memory.

### `crates/oriflamme-ai`: reusable strategy crate

Depends only on `oriflamme-core`.

- `trait Strategy: Send { fn choose(&mut self, view: &PlayerView, legal: &[Action], rng: &mut Rng64) -> Option<Action>; }`. A strategy gets a `PlayerView`, never a `GameState`, so it cannot read hidden cards.
- `RandomBot`, `HeuristicBot`, `MctsBot`; `Difficulty` and `make_strategy(difficulty)` map Easy, Medium, Hard to them.
- `heuristic.rs` works on a `GameState` but reads only the mover's own cards and public information. That lets `MctsBot` use it directly as the playout policy on determinized states. `HeuristicBot` determinizes once and calls it.
- `MctsBot` is flat Monte Carlo with UCB1 over the root actions: determinize, apply one candidate, play out with the heuristic, record the influence margin. The budget is a playout count, not time. `apply` (not `apply_traced`) is the hot path; keep it allocation-light.
- `tests/strength.rs` asserts heuristic beats random and search beats heuristic over fixed seeds. Re-run it after changing rules, heuristics or search.

### Root package `oriflamme-rs`: Bevy app (`src/`)

- UI is plain `bevy_ui`, rebuilt from scratch: `screens::rebuild_ui` despawns the `UiRoot` tree and respawns it whenever `UiDirty` is set or the `Screen` state changes. There is no incremental UI update; to change what is shown, change state and set `UiDirty`.
- Input: every clickable node carries a `Click` component. One global observer, `screens::on_click`, handles `On<Activate>` from `bevy::ui_widgets::Button`.
- `game::Session` (a resource that exists only once a game has started) owns the `GameState`, the log, and the human's in-progress selections. The human is always seat 0 (`HUMAN`).
- Playback: `Session::apply` calls `apply_traced` and turns each `Step` into one or more `Scene`s (`scenes_for` in `game.rs`): a frozen copy of the table, `Mark`s for the acting and affected cards, a sentence, and a duration. `play_scenes` advances them on a timer. While a scene is showing, `board` draws the scene's table instead of the live one, `human_turn()` is false, and `bot_turns` does nothing. Scene durations and labels live in `scenes_for`.
- Animation: a `Motion` component (`Pulse`, `Vanish`, `Arrive`, `Bob`) plus `screens::animate`, which drives `UiTransform` from a per-node clock. Clocks restart on every UI rebuild.
- Bots: `game::bot_turns` builds the strategy with `make_strategy` inside a task on `AsyncComputeTaskPool` and polls it with `check_ready`, so a long search does not block the window.
- Images are compiled into the exe with `include_bytes!` and decoded once into the `CardArt` resource (`screens.rs`). The game does not use the asset server for them, so the exe runs from any folder. Adding an image means adding it to `CardArt`.
- `main.rs` forces the DX12 backend on Windows unless `WGPU_BACKEND` is set: the Vulkan driver on the development machine's Intel GPU segfaults at start-up.
- Bevy is built with `default-features = false` and a UI-only feature list in `Cargo.toml`. A missing Bevy type usually means a missing feature there.
- `build.rs` embeds `assets/icon.ico` into the exe with `winresource`; `set_window_icon` in `icon.rs` (native only) sets the title-bar icon at run time through `WINIT_WINDOWS`.

### `crates/oriflamme-art`: art pipeline

`art/` holds hand-written SVG sources; `assets/` holds generated PNG and ICO files, which are committed. `cargo run -p oriflamme-art` rasterises `art/cards/*.svg` and `art/ui/*.svg` with `resvg` and writes `assets/icon.png` and `assets/icon.ico` from `art/icon.svg`. SVGs contain no text, because the rasteriser is built without fonts; card names are drawn by Bevy.

## Bevy 0.20 API

Bevy's API changes every release and differs from older tutorials (`children![]`, `ui_widgets::Button` with `Activate` observers, `FontSize::Px`, `border_radius` on `Node`, `UiTransform`). Check the 0.20.1 sources and examples in the local cargo registry (`~/.cargo/registry/src/*/bevy-0.20.1/examples`, `bevy_ui-0.20.1`) before writing against an API from memory.
