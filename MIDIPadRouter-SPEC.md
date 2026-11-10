# MIDIPadRouter (MIDIPR) — Specification for Claude Code

> Drop this file at the root of an empty folder (or rename it `CLAUDE.md`), then start Claude Code with: "Read SPEC and build the project step by step following the Milestones section. After each milestone, run the tests and show me the result."

## 1. Goal

**MIDIPadRouter** (short: **MIDIPR**) is an **ultra-lightweight** desktop application that lets you use **a single Novation Launchpad Mini MK1** with **several applications at the same time or alternately** (e.g. 2 radio programs), acting as a MIDI router/filter between the physical Launchpad and virtual MIDI ports that the target applications use as if they were "their own" Launchpad.

Audience: non-technical user (radio host). The interface must be obvious, with no manual.

## 2. Scope

- Hardware supported for v1: **Launchpad Mini MK1 only** (Novation, USB class-compliant, 8×8 pads + 8 top buttons + 8 side buttons). The code must isolate everything model-specific behind a `DeviceProfile` interface so other models can be added later, but **must not implement other models now**.
- Priority platform: **Windows 11**. macOS and Linux must build and work (same code base), but Windows is the test reference.
- Number of target applications: **1 to 5**.

## 3. Features

### 3.1 Two modes (selector at the top of the window)

**"Switch" mode**
- Only one target application is active at a time; it receives **all** the Launchpad's buttons and sends it its LEDs.
- The active target is changed by: a click in the UI, a configurable **global keyboard shortcut** (e.g. `Ctrl+Alt+1..5`), and a **dedicated Launchpad button** (default: top button no. 8, configurable, reserved: never forwarded to the applications).
- On target change: clear the Launchpad then **replay the stored LED state** of the new target (see 3.4), so the Launchpad immediately reflects its state.

**"Combined" mode**
- All applications are active simultaneously.
- Each Launchpad key (64 pads + 8 top buttons + 8 side buttons = **80 keys**) is **assigned to a single application** (or "none").
- An event on a key is sent only to the application that owns that key.
- LEDs sent by an application are forwarded to the Launchpad only for the keys it owns.

### 3.2 Key-by-key assignment (visual editor)
- Interactive grid reproducing the Launchpad Mini MK1 (8×8 + top row + right column).
- Each target application has **a color**; each key shows the color of its owner.
- Interactions: click = assign the key to the selected "brush" application; drag = paint several keys; right click = "none"; helper buttons: "Whole grid → application X", "Left / right half", "By column / by row", "Clear all".
- "**Mirror on the Launchpad**" option: the physical Launchpad briefly displays (or continuously when no application is writing to it) the color of each owner, to visualize the split.
- Undo / redo (Ctrl+Z / Ctrl+Y).

### 3.3 Target application management (1 to 5)
For each target: `name` (free text, e.g. "Radio A"), `color`, `port to the application` (receives the pads), `port from the application` (LEDs), enabled/disabled.
- Ports are chosen from a list of detected MIDI ports (refreshed live, see 3.6).
- **macOS/Linux**: "Create virtual ports" button (via `midir` virtual ports).
- **Windows**: `midir` cannot create virtual ports. The app shows a clear **wizard**: "Up-to-date Windows 11: use the built-in Windows MIDI Services loopback, otherwise install loopMIDI and create N port pairs named `MIDIPadRouter A in/out`, etc." with a link and automatic verification that the ports exist. The application must work with ports created by loopMIDI or by the Windows MIDI Services loopback — it only opens them.
- Reminder in the wizard: a loopback is a "pipe" — **two ports per target** are needed (one per direction) to avoid loops.

### 3.4 LED feedback and state memory
- The app keeps an **LED state buffer per target** (80 entries: current velocity/color) updated on every message received from the application, even when the target is not visible on the Launchpad (switch mode).
- In switch mode: on change, reset the Launchpad then send the new target's buffer.
- In combined mode: filter by owner; an LED message for a key that is not owned is ignored (but kept in the buffer).
- Also handle: reset message (`B0 00 00`), layout/buffer mode changes, which must be filtered or interpreted cleanly (never let an application put the Launchpad in "drum rack" mode or change the buffer without the internal state staying consistent — simplest: **block** these special messages and emulate their effect on the buffer).

### 3.5 Profiles / persistence
- Configuration as JSON in the OS config folder (`dirs`/Tauri path API).
- **Named profiles** (e.g. "Morning show", "Jingles"): mode, targets, assignment, shortcuts. Switch profile in one click.
- Profile import/export as a `.json` file.
- Automatic save, atomic write (temp file + rename).

### 3.6 Robustness
- Detection of Launchpad **plug/unplug** (light polling every ~2 s only while the Launchpad is absent, no polling otherwise) and automatic reconnection with state redisplay.
- Same for target ports: per-target state (🟢 connected / 🔴 port not found) in the UI.
- No panic/crash if a port disappears: the target goes into error, the rest keeps working.
- **No feedback loops**: never route an output to an input of the same group; validate the config on load.
- If the Launchpad's USB name contains "Launchpad Mini" but is not exactly MK1 (MK2/MK3 use other names/protocols), show an "Unsupported model" warning rather than misbehave.

### 3.7 Miscellaneous
- Notification-area icon: show the active target, change target/profile, quit.
- "Launch at Windows startup" and "Start minimized" options.
- Minimal MIDI monitor (collapsible panel): last ~50 frames with direction and target, for troubleshooting. **Disabled by default** and sends nothing to the front end while collapsed.
- Languages: **French by default**, English second (simple i18n files).

## 4. Launchpad Mini MK1 protocol (to be respected)

Reference: Novation "Launchpad Programmer's Reference" (MK1). Verify these points in the doc before implementing and put them in constants in `devices/launchpad_mini_mk1.rs` with tests.

- **8×8 pads** (default X-Y layout): Note On/Off channel 1; `note = 16 * row + column` (row 0 = top, column 0 = left). Press = Note On velocity 127, release = Note Off (or Note On velocity 0).
- **Side buttons (scene, right column)**: notes `8, 24, 40, 56, 72, 88, 104, 120` (column 8 of each row).
- **Top buttons**: Control Change `104..111` (0x68..0x6F), value 127 = press, 0 = release, channel 1.
- **LEDs**: send Note On on the same note with velocity = `16 * green + red + flags` (green and red from 0 to 3, flags = 12 for normal display, 8 for "copy" in double buffering, 0 for flashing depending on the mode). For top buttons: the matching CC with the same value.
- **Reset**: `B0 00 00`.
- Use a `Key` table (enum / index 0..79) with `Key <-> (type, MIDI number)` conversions tested by exhaustive unit tests (round trip over the 80 keys).

## 5. Tech stack (modern and light)

| Layer | Choice | Why |
|---|---|---|
| Desktop shell | **Tauri 2** | A few-MB binary, ~30–60 MB of RAM, uses the system WebView, no embedded Chromium |
| Core / MIDI | **Rust** + **`midir`** crate | Low latency, safe, no GC |
| Front end | **Svelte 5 (runes) + TypeScript + Vite** | Very light at runtime, reactive, no virtual DOM |
| Style | **Tailwind CSS 4** + CSS variables (auto light/dark theme) | Quick to write, modern look |
| Global shortcuts | Tauri `global-shortcut` plugin | |
| Tray / autostart | Tauri tray API + `autostart` plugin | |
| Persistence | `serde` + `serde_json` | |
| Tests | `cargo test` (core), **Vitest** (front end), MIDI integration tests with fake ports | |

Package manager: `pnpm`. No heavy UI library (no giant component framework): simple in-house components.

## 6. Architecture

```
src-tauri/
  src/
    main.rs               // Tauri bootstrap, tray, shortcuts
    app_state.rs          // shared state (Arc<...>), current config
    config.rs             // serde models + validation + version migrations
    devices/
      mod.rs              // DeviceProfile trait (keys, parsing, LED rendering)
      launchpad_mini_mk1.rs
    midi/
      ports.rs            // enumeration, opening, light hot-plug
      router.rs           // core: pad->target routing, LED->launchpad
      led_state.rs        // per-target LED buffers
      messages.rs         // MIDI parse/encode (typed, allocation-free on the hot path)
    commands.rs           // Tauri commands exposed to the front end
    events.rs             // events sent to the front end (throttled)
src/                      // Svelte front end
  lib/ (stores, i18n, components: PadGrid, TargetList, ModeSwitch, ProfileMenu, MidiMonitor, SetupWizard)
```

### Performance rules (mandatory)
1. **The MIDI hot path never goes through the front end**: routing runs entirely in Rust, inside the `midir` callbacks (no allocation, no long blocking lock; use `ArcSwap`/`parking_lot` to read the assignment config, a precomputed `[Option<TargetId>; 80]` table).
2. The front end is **notified only on useful state changes** (active target, connect/disconnect, per-target activity for a small indicator light), with **throttling ≤ 30 Hz**, and **nothing is sent while the window is hidden/minimized**.
3. **No continuous render loop** (no permanent `requestAnimationFrame`, no infinite animations); short CSS animations only.
4. No permanent polling: poll ports only if an expected port is missing, with backoff (2 s → 5 s).
5. Measurable targets to verify at the end: **CPU ≈ 0 % at idle**, < 1 % under heavy use, **RAM < 80 MB**, routing-added latency **< 1 ms** (measured in a test), installed binary < 15 MB.

## 7. Data model (versioned JSON config)

```jsonc
{
  "version": 1,
  "activeProfile": "default",
  "profiles": {
    "default": {
      "mode": "switch",              // "switch" | "combined"
      "activeTarget": "a",           // used in switch mode
      "switchKey": { "type": "cc", "number": 111 }, // reserved switch key (null = none)
      "shortcuts": { "a": "Ctrl+Alt+1", "b": "Ctrl+Alt+2" },
      "targets": [
        { "id": "a", "name": "Radio A", "color": "#3b82f6", "enabled": true,
          "portToApp": "MIDIPadRouter A in", "portFromApp": "MIDIPadRouter A out" }
      ],
      "assignment": { "pad:0": "a", "pad:1": "b", "cc:104": "a" } // key = Launchpad key, value = target id; absent = none
    }
  },
  "settings": { "language": "fr", "autostart": false, "startMinimized": false, "theme": "auto" }
}
```
Validation: max 5 targets, unique ids, no port used by two targets, no assignment to a nonexistent target, `switchKey` cannot be assigned to a target.

## 8. Interface (principles)

- A single window (~900×600, resizable), light/dark theme following the OS, rounded corners, system typography, generous spacing, AA contrast.
- Top bar: **mode selector** (2 large segments "Switch" / "Combined"), profile selector, Launchpad status (🟢/🔴).
- Central left area: **interactive Launchpad grid** (SVG/CSS grid, no canvas). In switch mode: the grid shows the Launchpad with the active target's LED state; in combined mode: the colored assignment editor.
- Right area: **target list** (cards with name, color, port status, activity indicator, "Active" button in switch mode, "+ Add" button up to 5).
- First-launch wizard in 3 steps: (1) detect the Launchpad Mini MK1, (2) create/choose the applications' ports, (3) choose the mode. Must be rerunnable from the settings.
- Error messages in plain language ("The Radio B application is not listening on this port", etc.), never raw jargon.
- Keyboard accessible, ARIA labels on the grid.

## 9. Tests and acceptance criteria

**Unit tests (Rust)**: conversion of the 80 keys ↔ MIDI messages; LED velocity encoding; filtering by owner; LED memory/replay on target change; config validation; blocking of special messages.
**Integration tests**: router with fake MIDI ports (mockable `MidiIn`/`MidiOut` traits) — simulate a pressed pad in switch/combined mode, LED feedback, unplugging a target.
**Front-end tests (Vitest)**: stores, assignment painting, undo/redo, i18n.

Acceptance criteria:
1. In combined mode, 2 targets: a key assigned to A **never** reaches B, and vice versa.
2. In switch mode: after a target change, the Launchpad shows the new target's state in < 100 ms.
3. Targets keep working if one of them loses its port.
4. App restart: config and profile restored identically.
5. Performance targets of section 6 met (measurements recorded in `PERF.md`).

## 10. Milestones (work in this order, commit at each milestone)

1. **Skeleton**: Tauri 2 + Svelte 5 + TS + Tailwind project, lint/format (clippy, rustfmt, eslint, prettier), GitHub Actions CI (build + tests on Windows).
2. **MIDI core**: `DeviceProfile` + Launchpad Mini MK1, port enumeration/opening, console log of what the Launchpad sends. Protocol unit tests.
3. **Switch-mode router** + LED buffer + replay on change + keyboard/dedicated key.
4. **Combined-mode router** + assignment table + LED filtering by owner.
5. **Config/profiles/persistence** + validation + migrations.
6. **UI**: interactive grid, painting editor, target list, mode selector, profiles.
7. **Port setup wizard** (Windows: loopMIDI / Windows MIDI Services loopback detection; macOS/Linux: virtual ports).
8. **Tray, global shortcuts, autostart, MIDI monitor**, hot-plug.
9. **FR/EN i18n, accessibility, visual polish**, app icon.
10. **Performance measurements** (`PERF.md`), packaging (Windows MSI/NSIS installer), README in English with screenshots and a "how to connect my 2 radio programs" guide.

## 11. Out of scope (v1)
Other Launchpad models (MK2/MK3/Pro/X), note transformation/remapping, multiple MIDI channels, user scripts, cloud sync, virtual port creation on Windows (depends on loopMIDI or the Windows MIDI Services loopback).

## 12. Instructions for Claude Code
- Before coding the Launchpad protocol, **check the official MK1 doc** (WebSearch/WebFetch) and fix section 4 if needed.
- If a decision is not covered here, pick the simplest and lightest, and note it in `DECISIONS.md`.
- No unnecessary dependencies; justify each new crate/package.
- Write code commented in English, a UI in French (i18n).
- Never route MIDI traffic through the front end.
