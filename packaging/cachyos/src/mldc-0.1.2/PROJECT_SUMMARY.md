# Project Summary: Minimal Linux Disk Cloner

## Overview

This project is a **Linux-only desktop disk cloning tool** built with **Rust + egui**.
Its purpose is to provide a **very lightweight graphical wrapper around `dd`** for direct **disk-to-disk cloning**, while keeping the process simple, fast, and transparent.

The app is **not** a backup suite, partition editor, or imaging platform. It exists to do one job well:

**clone one block device directly onto another block device using `dd`.**

The main value of the app is not replacing `dd`, but making the workflow safer and easier by:

* clearly listing source and target disks
* showing device metadata in a readable way
* warning about obvious risks
* requiring an explicit confirmation step
* showing live cloning progress
* presenting a clean success or failure screen at the end

The design philosophy is:

* minimal steps
* minimal dependencies
* no unnecessary abstraction
* no filesystem-level interpretation
* no attempt to “improve” `dd` by becoming something else

This is effectively a **thin, intentional GUI for a dangerous but useful Linux workflow**.

---

# Core Goal

Provide a fast, lightweight, Linux-native app that allows a user to:

1. select a source disk
2. select a target disk
3. review a summary with warnings
4. start cloning
5. watch real progress
6. see a final result screen

The app should behave in a way that respects how `dd` works:

* it should warn when something looks wrong
* it should not overprotect the user
* it should not silently “fix” things
* it should not block smaller target disks purely because the copy is risky

If the user wants to do something stupid, the app should make that obvious, not impossible.

---

# Platform and Tech Stack

## Target platform

* Linux only

## Language

* Rust

## UI framework

* egui / eframe

## System interaction

* `std::process::Command` for calling system tools
* `dd` for cloning
* `lsblk` for device discovery
* optionally `/sys/block` or `udevadm` for extra device metadata if needed later

## Primary runtime assumptions

* `dd` is available on the system
* `lsblk` is available on the system
* app is run with sufficient permissions, or the actual clone command is executed through privilege escalation

---

# What the App Is

The app is a **focused graphical disk cloner** for Linux users who want:

* a GUI
* fewer chances to choose the wrong disk by accident
* live progress while cloning
* a cleaner workflow than manually typing `dd`

It is intended for:

* cloning Linux installs
* cloning drives during hardware replacement
* copying one disk to another for migration or duplication
* advanced home users, hobbyists, Linux tinkerers, repair users, and technicians

It is **not intended** to be a beginner-proof toy.
It is allowed to be a serious tool for people who understand that disk cloning is destructive.

---

# What the App Should Do

## Functional Scope

### 1. Detect available disks

The app should detect physical or relevant block devices and present them clearly.

It should gather and show:

* device path, e.g. `/dev/sda`, `/dev/nvme0n1`
* size
* model
* serial if available
* transport/type if available
* removable flag if available
* mounted partitions if any
* whether the device is currently in use or mounted

This detection should preferably be based on:

```bash
lsblk -J -o NAME,PATH,SIZE,MODEL,SERIAL,TYPE,RM,MOUNTPOINT,FSTYPE
```

The app should focus on devices where `TYPE=disk`.

### 2. Let the user choose a source disk

The user selects a source device from the disk list.

The UI should make it clear:

* this is the disk being read from
* no data on the source is modified by the clone itself
* cloning a mounted source may produce inconsistent results

### 3. Let the user choose a target disk

The user selects a target device from the disk list.

The UI should make it clear:

* this disk will be overwritten
* all target data will be destroyed
* mounted target partitions are risky and should be warned about

### 4. Prevent impossible or nonsensical selection

The app must not allow:

* source and target being the same device

This is not a “safety preference”, it is basic sanity.

### 5. Show warnings, not hard blocks

The app should warn, but generally still allow proceeding, for things like:

* target smaller than source
* source has mounted partitions
* target has mounted partitions
* target appears to already contain data or partitions
* removable drive selected
* unknown device metadata

The only real hard-stop case should be invalid execution state such as:

* no source selected
* no target selected
* source equals target
* required system tools unavailable

### 6. Show a summary screen

Before cloning starts, the app should show a dedicated review screen with:

* source device
* target device
* source size
* target size
* warnings
* exact direction of the operation in very clear form:

  * `SOURCE -> TARGET`
* the actual `dd` command that will be used
* destructive warning that all target data will be overwritten

The call to action should be explicit:

* `Start Cloning`

Optional but recommended:

* require one extra confirmation action like a checkbox:

  * “I understand the target disk will be overwritten”

No theatrical nonsense. Just one deliberate friction point.

### 7. Execute cloning with dd

The app should start cloning using `dd` as a subprocess.

Recommended command:

```bash
dd if=/dev/SOURCE of=/dev/TARGET bs=64K conv=fsync status=progress
```

### 8. Show live progress

The app should show progress while cloning is running.

Progress should include:

* bytes copied
* total source bytes
* percentage
* current throughput if derivable
* elapsed time
* estimated time remaining if derivable
* textual log/output stream

This can be done by parsing `dd`’s progress output from stderr.

### 9. Handle completion or failure

When cloning finishes, the app should show a final result state:

Success:

* clone completed
* total bytes copied
* elapsed time
* average speed
* note that the OS may need to re-read partition tables or the drive may need reconnecting

Failure:

* clone failed
* show error output
* show partial progress if known
* make the result readable, not cryptic

---

# What the App Should Not Do

These are deliberately out of scope for v1:

* image file creation
* image file restoration
* compression
* filesystem-aware cloning
* partition resizing
* post-clone expansion
* checksum verification
* selective partition cloning
* backup scheduling
* cloud sync
* rescue environment
* Windows support
* macOS support
* advanced device editing
* partition management
* secure wipe mode
* multithreaded “optimized” cloning engine

This project should stay focused on **block device to block device cloning with dd**.

---

# UX Flow

## Step 1: Source Selection

Screen title example:

* `Choose Source Disk`

Display a scrollable list of detected disks with enough detail to distinguish them:

* path
* model
* size
* serial
* mounted partitions summary

User selects one disk.

CTA:

* `Continue`

---

## Step 2: Target Selection

Screen title example:

* `Choose Target Disk`

Same disk list format.

The UI should visually emphasize:

* this disk will be overwritten

If target is smaller than source, show warning immediately but still allow selection.

CTA:

* `Continue`

---

## Step 3: Summary / Confirmation

Screen title example:

* `Review Clone Operation`

Display:

* Source: full disk info
* Target: full disk info
* warnings block
* command preview
* destructive notice
* large copy direction row

Example:

```text
Source: /dev/nvme0n1 - Samsung 1 TB
Target: /dev/sdb - Kingston 960 GB

Operation:
Read from /dev/nvme0n1
Write to   /dev/sdb

Warnings:
- Target is smaller than source
- Target has mounted partitions
- All data on target will be overwritten
```

CTA:

* `Start Cloning`

Optional secondary:

* `Back`

---

## Step 4: Progress

Screen title example:

* `Cloning in Progress`

Display:

* progress bar
* percent
* copied bytes / total bytes
* current speed
* elapsed time
* estimated remaining time
* scrolling output log

Optional:

* disable closing while active, or warn if trying to close
* allow cancellation only if you’re prepared to handle termination cleanly

For v1, it is acceptable to avoid a cancel button if you want to keep behavior simpler.

---

## Step 5: Result

Screen title example:

* `Finished`

Display either success or failure.

Success:

* “Clone completed successfully”
* stats
* maybe button for `Clone Another Disk`

Failure:

* “Clone failed”
* stderr/log output
* maybe button for `Back to Start`

---

# System Behavior and Rules

## Device Discovery Rules

Use `lsblk` JSON output and parse it into Rust structs.

Prefer showing only top-level block devices where:

* `type == "disk"`

Hide or ignore by default:

* loop devices
* zram
* ram devices
* partitions as top-level choices

The user selects whole disks, not partitions.

## Mounted State Rules

Mounted devices should be detected and surfaced.

Behavior:

* if source has mounted partitions, warn
* if target has mounted partitions, warn strongly
* do not silently unmount anything in v1
* do not automatically block cloning because of mounted partitions

## Source / Target Rules

Must reject:

* identical source and target

Must warn:

* target size smaller than source size

Should allow:

* smaller target despite warning

## Privilege Rules

Cloning requires elevated privileges.

You have two possible models:

### Simple v1 model

Run the whole app as root or via `pkexec`.

Pros:

* simple implementation

Cons:

* entire GUI runs elevated

### Better long-term model

Run UI unprivileged and execute only clone operation through privileged helper.

Pros:

* cleaner architecture
* safer

Cons:

* more work

For v1, the simple model is acceptable.

---

# Recommended Command Strategy

Base clone command:

```bash
dd if=/dev/SOURCE of=/dev/TARGET bs=64K conv=fsync status=progress
```

## Notes

* `if` = source block device
* `of` = target block device
* `bs=64K` is a sensible default
* `conv=fsync` helps flush writes properly
* `status=progress` provides live progress output

You may later make `bs` configurable, but not in the first version.

---

# Progress Handling

## Source of truth

Use the source device size from `lsblk` as the total expected byte count.

## Progress parsing

Parse `dd` progress output from stderr.

Typical output includes copied bytes and transfer rate.
You can extract:

* bytes copied
* transfer speed
* elapsed state from local timer
* remaining estimate using bytes copied vs total

## UI representation

Show:

* progress bar from 0.0 to 1.0
* textual stats
* raw log area

Even if ETA is rough, that is better than a dead screen with no feedback.

---

# Error Handling Expectations

The app should fail clearly when:

* `dd` is missing
* `lsblk` is missing
* permission is denied
* source or target device disappears
* process exits non-zero
* output cannot be parsed fully

The UI should not panic or crash just because progress parsing is imperfect.
If needed, fall back to:

* showing raw output only
* indeterminate progress state

---

# Suggested Internal Architecture

## Modules

### `main.rs`

App startup and eframe bootstrap.

### `app.rs`

Main egui app state and top-level screen routing.

### `models.rs`

Shared structs such as:

* `DiskDevice`
* `PartitionInfo`
* `CloneConfig`
* `CloneProgress`
* `CloneResult`
* `WarningItem`

### `device.rs`

Disk discovery and parsing `lsblk` output.

Responsibilities:

* call `lsblk`
* parse JSON
* filter valid disks
* derive warnings and mount info

### `clone.rs`

Clone execution and progress parsing.

Responsibilities:

* build `dd` command
* spawn process
* read stderr/stdout
* update progress state
* return success/failure result

### `warnings.rs`

Encapsulate warning generation logic.

Examples:

* target smaller than source
* mounted partitions found
* removable disk selected

### `ui/`

Optional folder if you want cleaner separation for screen rendering:

* `source_screen.rs`
* `target_screen.rs`
* `summary_screen.rs`
* `progress_screen.rs`
* `result_screen.rs`

Not required, but helpful if the file grows.

---

# Suggested Data Structures

## DiskDevice

Represents a cloneable disk.

Fields could include:

* path: String
* name: String
* size_bytes: u64
* size_human: String
* model: Option<String>
* serial: Option<String>
* removable: bool
* mounted_partitions: Vec<PartitionInfo>
* transport: Option<String>

## PartitionInfo

Fields:

* path: String
* mountpoint: Option<String>
* fstype: Option<String>

## CloneConfig

Fields:

* source: DiskDevice
* target: DiskDevice
* block_size: String
* command_preview: String

## CloneProgress

Fields:

* bytes_copied: u64
* total_bytes: u64
* percent: f32
* speed_bytes_per_sec: Option<u64>
* elapsed: Duration
* eta: Option<Duration>
* raw_output: Vec<String>

## CloneResult

Fields:

* success: bool
* bytes_copied: u64
* elapsed: Duration
* average_speed: Option<u64>
* final_message: String
* raw_output: Vec<String>

---

# UI Style Direction

Because it uses egui, the UI should be:

* clean
* compact
* readable
* functional first
* not over-designed
* dark mode friendly

Think:

* clear panels
* prominent disk cards
* warning boxes
* single main CTA per step
* no decorative clutter

This app is closer to a utility than a “productivity experience.”
No one needs floating glassmorphism to copy `/dev/nvme0n1` to `/dev/sdb`. Humanity has suffered enough.

## Visual priorities

* source and target should look distinct
* destructive action should be visually obvious
* progress should feel alive
* final result should be unambiguous

---

# Safety Philosophy

The app should be **careful, not parental**.

That means:

* warn clearly
* show exact device paths
* show exact command
* require explicit confirmation
* prevent obviously nonsensical selections

But:

* do not refuse smaller targets purely on principle
* do not hide what the tool is doing
* do not pretend it is safer than `dd`
* do not silently adjust behavior

This should feel like a trustworthy frontend for a dangerous tool, not a toy pretending danger does not exist.

---

# Future Enhancements, But Not v1

Only after the minimal version works well:

## Nice future ideas

* image file mode
* optional verification pass
* optional auto-unmount target partitions
* block size setting
* saved logs
* smart target filtering
* external drive highlighting
* post-clone hints for resizing partitions/filesystems
* privilege helper separation

## Still avoid even later unless there is a very good reason

* turning it into full backup software
* filesystem-specific logic explosion
* cross-platform abstraction
* plugin systems
* “AI assistance,” because apparently every object in the universe must now pretend to think

---

# Product Positioning

This tool should be positioned as:

> A tiny Linux GUI for raw disk-to-disk cloning with `dd`.

Not:

* “backup solution”
* “disk migration suite”
* “data protection platform”
* “universal media writer”

That clarity will help both the codebase and the user expectations.

---

# Short Build Brief for Codex / Cursor

Build a Linux-only Rust desktop app using `eframe/egui` that acts as a minimal GUI wrapper around `dd` for disk-to-disk cloning.

Requirements:

* detect disks using `lsblk -J`
* show only valid top-level disks
* allow selecting source and target
* block same-device selection
* warn if target is smaller than source
* warn if source or target has mounted partitions
* summary screen before execution
* run `dd if=SOURCE of=TARGET bs=64K conv=fsync status=progress`
* parse progress output and show progress bar, bytes copied, percent, speed, elapsed, ETA
* final success/failure screen with logs
* no image-file support
* no partition manager
* no auto-unmount in v1
* clean minimal egui UI
* Linux only

Architecture should separate:

* device discovery
* clone execution
* progress parsing
* UI state/screens

---

# Practical Summary

This project is a **small, focused Linux disk cloner GUI** built in **Rust + egui** that wraps `dd` for **raw disk-to-disk cloning**.

It should:

* discover disks clearly
* let the user choose source and target
* warn about risky conditions
* show a clear confirmation step
* run `dd`
* display real progress
* end with a clean result screen

It should not:

* become a backup suite
* become a partition manager
* try to be clever
* hide the underlying operation

The whole point is to keep the tool:

* fast
* readable
* trustworthy
* minimal

And that, for once, is enough.
