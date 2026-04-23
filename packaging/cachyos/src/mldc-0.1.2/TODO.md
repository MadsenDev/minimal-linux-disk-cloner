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
- [x] Embed the tracked main screenshot in `README.md`
- [x] Generate the packaged app icon set from the existing SVG logo
- [x] Build the optimized Tauri desktop binary with the updated icon assets
- [x] Add a native CachyOS/Arch pacman packaging recipe
- [x] Add a local helper to build a `.pkg.tar.zst` package from the current working tree
- [x] Harden the packaged launcher for KDE Plasma Wayland startup
- [x] Add a Linux launcher elevation prompt so packaged app startup can request root through Polkit instead of requiring `sudo mldc`
- [x] Fix the CachyOS package helper so it excludes generated `packaging/cachyos/src` and `pkg` trees when creating the source tarball
- [x] Fix mount detection so unmounted USB partitions do not display or warn as mounted
- [x] Make the smaller-target warning show precise sizes when rounded GiB labels look identical
- [x] Fix clone progress streaming so `dd status=progress` updates render live instead of only after stop or completion
- [x] Bump the project version from `0.1.0` to `0.1.1` across app, package, and docs metadata
- [x] Redesign the settings modal into a tabbed preferences surface for general, safety, and advanced options
- [x] Add a persisted safety toggle that blocks clone start when `Danger` warnings are present
- [x] Add a persisted power-user mode with advanced detail toggles for logs, byte precision, and warning diagnostics
- [x] Pin the Tauri dev server and dev URL to IPv4 loopback so dev mode does not white-screen on systems where `localhost` resolves to IPv6 only
- [x] Add active animation to the progress bar so clone and verify progress feels alive between streamed updates
- [x] Apply the KDE Plasma WebKit DMABUF workaround to `tauri dev` scripts so development windows do not white-screen even outside the packaged launcher
- [x] Reduce packaged UI sluggishness by isolating the hot progress/result render paths and trimming the most expensive always-on visual effects
- [x] Replace the native Linux titlebar with a custom MLDC titlebar that keeps drag, double-click maximize/restore, minimize, maximize/restore, and close behavior
- [x] Make the custom titlebar span the full window width and reorder it into left-side app actions, centered branding, and right-side window controls
- [x] Bump the project version from `0.1.1` to `0.1.2` across app, package, and docs metadata
- [x] Keep this list updated while work is in progress

## Notes

- Add new tasks before starting substantial work.
- Mark tasks complete when the work is actually done.
- Remove or archive stale items when they no longer matter.
