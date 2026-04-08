# TODO

Use this file as the working checklist for the repository.

## Active

- [x] Read `PROJECT_SUMMARY.md`
- [x] Create `TODO.md`
- [x] Create `CHANGELOG.md`
- [x] Create `AGENTS.md`
- [x] Scaffold Rust project structure
- [x] Implement disk discovery from `lsblk`
- [x] Implement warning generation
- [x] Implement `dd` clone execution and progress parsing
- [x] Implement egui multi-step flow
- [x] Add Linux desktop packaging assets
- [x] Build and verify the project
- [x] Redesign the UI into a premium industrial shell
- [x] Rework screen layouts for source, target, review, progress, and result
- [x] Re-verify formatting, tests, and release build after redesign
- [x] Simplify the UI into a centered wizard flow
- [x] Hide secondary command and log details behind expandable sections
- [x] Re-verify formatting, tests, and release build after wizard redesign
- [x] Fix wizard layout regression hiding device cards
- [x] Make wizard use the full window width and top-aligned flow
- [x] Remove outer page scrolling that is creating phantom vertical space
- [x] Fix step indicator consuming remaining page height
- [x] Replace staged setup UI with a single centered dropdown screen
- [x] Reshape the setup screen into a horizontal Etcher-style source-target-clone layout
- [x] Re-center the Etcher-style setup row around the target stage
- [x] Replace approximate setup centering with explicit composition centering math
- [x] Migrate the desktop shell from `eframe/egui` to Tauri + React
- [x] Expose Rust disk discovery, warning generation, and clone execution via Tauri commands/events
- [x] Rebuild the UI with Vite, Tailwind, icons, and motion
- [x] Simplify the Tauri frontend toward a flatter balenaEtcher-style presentation
- [x] Replace native selects with custom device picker popovers
- [x] Add a backend-driven fake-disk simulation mode for development
- [x] Lock the setup screen geometry so warnings do not shift the top row
- [x] Apply an Etcher-plus visual overhaul without changing screen geometry
- [x] Add an optional post-clone verification pass with a second live progress phase
- [x] Add a guarded stop control for in-progress clone and verification runs
- [x] Add run report export after completion
- [x] Revalidate selected devices immediately before cloning starts
- [x] Replace native confirmation prompts with custom in-app modals
- [x] Portal modals to `document.body`
- [x] Remove remaining Etcher references from app-facing copy
- [x] Add a gentle animated background that respects reduced motion
- [x] Redesign the animated background into a cleaner layered backdrop
- [x] Improve Flash button contrast between disabled and enabled states
- [x] Remove redundant clone/disk wording from the header strapline
- [x] Rewrite `README.md` into a polished GitHub-facing project page
- [x] Make the current project version explicit in the public docs
- [x] Add a settings modal next to Refresh with persisted UI preferences
- [x] Keep this list updated while work is in progress

## Notes

- Add new tasks before starting substantial work.
- Mark tasks complete when the work is actually done.
- Remove or archive stale items when they no longer matter.
