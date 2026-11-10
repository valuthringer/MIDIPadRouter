# Decisions

Choices not covered (or adjusted) by the specification, from most to least important.

## Launchpad Mini MK1 protocol (verification of section 4)

- Verified against the *Launchpad Programmer's Reference*, the `launchy` crate and
  `Athaphian/web-midi-launchpad` (`(flashing ? 8 : 12) | green << 4 | red`).
- Velocity = `16 × green + red + flags`, green and red from 0 to 3. Bit 3 (8) = *Clear* (clears
  the LED in the other buffer), bit 2 (4) = *Copy* (writes the LED to both buffers).
- **Spec correction**: the LED flags are `12` = normal display (Clear + Copy), `8` = **flashing**
  (Clear only: lit in one buffer, off in the other, alternated by `B0 00 28`), `0` =
  **double buffering** (writes only the update buffer). The spec swapped "copy" and "flashing".
- Constants and tests are in `src-tauri/src/devices/launchpad_mini_mk1.rs`.
- The key index order (0..63 grid, 64..71 side, 72..79 top) is that of the manual's *rapid update*
  (`92h`): decoding an application's rapid updates becomes trivial.
- Special messages sent by applications:
  - `B0 00 00` (reset): **emulated** on the application's buffer; only its visible keys are turned off.
  - `B0 00 01/02` (X-Y layout / drum rack), buffer control other than `20h`/`28h`, LED test
    `7Dh..7Fh`, duty cycle (CC 30/31), SysEx, other channels: **blocked**.
  - `B0 00 20` / `B0 00 28` (automatic flashing off/on, buffer 0): forwarded, with no effect on
    state consistency.
  - `92h` (rapid update): decoded into individual LEDs, filtered by owner.
- Replay on target switch: `B0 00 00` then 40 rapid update messages (80 LEDs).
- "Launchpad Mini" is accepted (MK1 and Mini MK2, same protocol). Any other name containing
  "Launchpad", "MK3", "LPMini", "LPX", "LPPro" shows "Unsupported model".

## Architecture

- **A single std `Mutex`** around the router instead of `ArcSwap`/`parking_lot`: MIDI traffic is
  tiny, the lock is held only for a few `send` calls (measured < 1 µs per message, see `PERF.md`)
  and it avoids two dependencies. Input connections are always closed with the lock released
  (closing one waits for the callback that takes this lock).
- `led_state.rs` and `messages.rs` from the proposed tree do not exist: the LED buffer is a
  `[u8; 80]` in `Target`, and MIDI parsing/encoding is inherently model-specific, so it lives in
  the `DeviceProfile`. `tray.rs` is split from `main.rs` for readability.
- UI notifications: bounded channel (`sync_channel(1)`, `try_send`, no allocation) waking a thread
  that waits 33 ms (≤ 30 Hz), then emits a full snapshot only if the window is visible and not
  minimized. When the window comes back, the front end requests the state again.
- `activeTarget` belongs to the backend (Launchpad key, shortcut, tray): it is persisted by the UI
  thread, off the hot path, and a save coming from the front end does not overwrite it.

## Robustness

- **Deliberate deviation from spec 3.6**: when everything is connected, the port list is re-read
  every **5 s** (not only when a port is missing). Without this, a Launchpad unplugged then
  replugged with no LED traffic in between stays silent forever (midir does not report
  unplugging). Measured cost: negligible. When a port is missing: 2 s then 5 s.
- New **"Launchpad busy"** status: the Launchpad is present but cannot be opened (a radio program
  still uses it directly — the most common case on Windows).
- Unreadable/invalid config at startup: renamed to `config.invalid.json` (never deleted), default
  configuration loaded, message shown to the user.
- `single-instance` plugin: a 2nd instance would fight for the Launchpad; it brings the 1st one to
  the foreground.
- Closing the window **destroys the WebView** (≈ 190 MB freed, see `PERF.md`); routing continues
  in the Rust core and the tray icon stays. "Quit" is in the icon's menu. The window is recreated
  on open (icon, 2nd launch, unless "Start minimized").
- Global shortcuts are registered only in Switch mode (they only serve to change target): in
  Combined mode the keys stay free for other programs.
- Assignment keys: `pad:<MIDI note>` and `cc:<number>`, like `switchKey` (`type` = `pad`|`cc`).

## Interface

- The brush is chosen in the grid toolbar; "by column / by row" is a **brush shape** (one click
  paints the whole row/column).
- The switch key is chosen among the 16 round buttons (top and side); it is only used in Switch
  mode but stays reserved in both modes (validation).
- "Mirror on the Launchpad" is a toggle: while it is on, the Launchpad shows each key's owner
  (approximate Launchpad colors: green, red, amber, yellow, orange), and the applications' LEDs
  stay memorized and are replayed afterwards.
- In Switch mode, the switch key lights up in the Launchpad color of the active target.
- Virtual ports (macOS/Linux): global `virtualPorts` setting; names `MIDIPadRouter X in/out`.
- Help links are opened by a Rust command with fixed URLs (no "open any URL" permission on the web
  side).

## Dependencies

Rust:
- `tauri` (+ `tray-icon`): shell required by the spec.
- `tauri-plugin-global-shortcut`, `tauri-plugin-autostart`: required by the spec.
- `tauri-plugin-dialog`: file picker for profile import/export.
- `tauri-plugin-opener`: open the loopMIDI / Windows MIDI Services pages.
- `tauri-plugin-single-instance`: see Robustness.
- `midir`, `serde`, `serde_json`: required by the spec.

JS (dev only, except `@tauri-apps/api`): Svelte 5, Vite, TypeScript, Tailwind 4, Vitest, ESLint,
Prettier, svelte-check — required by the spec. TypeScript is pinned to 6.0 because
`typescript-eslint` does not support TypeScript 7 yet.
