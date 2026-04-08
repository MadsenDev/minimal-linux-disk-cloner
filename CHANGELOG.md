# Changelog

All notable repository changes should be recorded here.

## Unreleased

### Added

- Added `TODO.md` for a checkable working task list.
- Added `CHANGELOG.md` to track notable repository changes.
- Added `AGENTS.md` with instructions to check and update both files during work.
- Added a new Rust `eframe/egui` desktop application scaffold for Minimal Linux Disk Cloner.
- Added Linux disk discovery via `lsblk` JSON parsing with filtering to top-level cloneable disks.
- Added warning generation for destructive and risky clone conditions.
- Added `dd` clone execution with live progress parsing, log capture, and result handling.
- Added a multi-step GUI flow for source selection, target selection, review, progress, and results.
- Added Linux desktop packaging assets including a `.desktop` file and SVG icon placeholder.
- Added build and usage documentation in `README.md`.
- Added an optional `Verify after clone` workflow with a full streamed byte-for-byte verification pass.
- Added a guarded stop action for active clone and verification runs.
- Added run report export that writes a text report for completed runs.

### Changed

- Planning a premium industrial redesign pass for the app shell and screen layouts while keeping the existing clone workflow intact.
- Reworked the app UI into a premium industrial operator-console layout with a branded top bar, persistent step rail, atmospheric background treatment, richer cards, and stronger status surfaces.
- Redesigned the source, target, review, progress, and result screens to create clearer focal points and more deliberate visual hierarchy without changing the underlying clone workflow.
- Planning a follow-up simplification pass to replace the operator-console layout with a cleaner Etcher-style wizard and reduced default detail density.
- Replaced the operator-console presentation with a centered three-step wizard flow closer to balenaEtcher-style simplicity.
- Reduced default information density by hiding the raw command preview and run logs behind expandable sections while keeping the existing clone behavior unchanged.
- Investigating a follow-up wizard layout regression where device discovery still succeeds but device cards may not render visibly.
- Fixed the centered wizard width allocation so detected devices render as full-width cards again instead of disappearing behind collapsed layout constraints.
- Adjusting the wizard container again so it fills the available window width and stays top-aligned instead of reading like a narrow floating column.
- Removed the last fixed-size wizard wrapper so the app now lays out top-down and uses the available window width instead of leaving most of the screen empty.
- Removing the outer page scroll container as a follow-up, since it appears to be introducing phantom vertical space above the wizard card.
- Removed the outer page scroll container so the wizard now renders in a normal top-down flow instead of pushing the main card far below the visible header and step indicator.
- Tracing another layout issue in the simplified wizard where the step indicator appears to be consuming too much vertical space and pushing the main card off-screen.
- Replaced the centered step-indicator row with a normal horizontal flow so it no longer consumes the remaining page height and push the wizard card out of view.
- Replacing the staged setup flow with a single centered source/target/clone screen to match the simpler balenaEtcher-style interaction model requested by the user.
- Removed the remaining staged setup code paths and now drive setup through one centered panel with source and target dropdowns plus a single Clone action.
- Reworking the new single-screen setup again so it uses a more obviously Etcher-like horizontal source-target-clone composition rather than a centered form card.
- Reshaped the setup screen into a horizontal three-stage layout with centered source, target, and clone controls to better match the balenaEtcher interaction style the user wanted.
- Re-centered the horizontal setup composition so the target stage is the actual visual midpoint, with source and clone balanced evenly to either side.
- Replaced the setup row's approximate centering behavior with explicit composition centering so the source-target-clone strip is positioned by exact width rather than by nested container heuristics.
- Migrated the app from an `eframe/egui` shell to a Tauri desktop app with a React, Vite, Tailwind, and Framer Motion frontend.
- Moved the Rust disk discovery, warning generation, and `dd` execution logic under `src-tauri` and exposed it through Tauri commands plus clone progress/result events.
- Rebuilt the setup, progress, and result flows as a polished Etcher-style web UI with icons, animated transitions, and exact layout control.
- Pulled the frontend back to a flatter, simpler balenaEtcher-style presentation with less chrome, fewer panels, and more literal source-target-flash staging.
- Replaced the native device `<select>` controls with custom popover pickers that show disk cards, metadata badges, and clear selection state.
- Added an explicit fake development mode with backend-provided fake disks and simulated clone progress so the full app can be exercised without root or real block devices.
- Locked the setup screen geometry so the source-target-flash row stays in the same position whether the warnings area is populated or empty.
- Applied an Etcher-plus visual overhaul on top of the locked layout using a dark slate and orange design system, tighter surface styling, and subtle motion across setup, progress, and result.
- Extended clone progress and result events with phase-aware metadata so the app can distinguish cloning from verification.
- Updated the setup, progress, result, and fake-mode flows to support an optional verification phase without changing the locked setup layout.
- Added a pre-flight device revalidation step immediately before a clone starts so stale selections fail cleanly instead of starting with outdated device state.
- Updated the progress and result screens to surface the new stop and save-report controls without changing the fixed setup layout.
- Replaced the remaining browser-native stop confirmation prompt with a custom in-app modal to keep dialogs visually consistent with the rest of the UI.
- Moved the confirmation modal to a `document.body` portal so it is no longer constrained by the app shell stacking context.
- Removed the remaining Etcher-specific references from the app header and README so the UI copy is fully product-specific.
- Added a gentle CSS-driven background drift animation and disabled it for reduced-motion users.
- Reworked the background again into a cleaner layered backdrop with separate warm and cool ambient fields, a subtler grid, and a stronger vignette so the screen reads less muddy.
- Increased the visual separation between the disabled and active Flash button states so availability is clearer at a glance.
- Refined the header strapline to remove redundant clone/disk wording while keeping the product name unchanged.
- Rewrote `README.md` into a more complete GitHub-facing project page with clearer product positioning, safety notes, runtime requirements, and fake-vs-real development workflow guidance.
- Made the current project version explicit in the README while keeping the app manifests aligned at `0.1.0`.
- Added a settings modal beside Refresh with persisted preferences for default verification and background motion, plus basic runtime/app info.
- Embedded the tracked main application screenshot at the top of the README.
