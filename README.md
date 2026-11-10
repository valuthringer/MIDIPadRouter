# MIDIPadRouter

**One Launchpad Mini, several applications.** MIDIPadRouter (MIDIPR) sits between your
Novation Launchpad Mini MK1 and your software (for example two radio automation programs). Each
program believes it has "its own" Launchpad.

- **Switch mode**: one application at a time receives the whole Launchpad. Change with the ⇄ key
  on the Launchpad (the round top-right button by default), a keyboard shortcut (`Ctrl+Alt+1`,
  `Ctrl+Alt+2`…) or a click. The Launchpad immediately redisplays the lights of the selected
  application.
- **Combined mode**: all applications at once; you "paint" each key with the color of the
  application that receives it.

Lightweight app (Tauri + Rust): about 0 % CPU when idle.

## Installation

Download the Windows installer (`MIDIPadRouter_x.y.z_x64-setup.exe` or `.msi`) and run it.
macOS and Linux: build from source (see below).

## How to connect my 2 radio programs (Windows)

Each program needs **two MIDI "pipes"**: one to receive the pads, one to send the lights back.
MIDIPadRouter cannot create these pipes itself on Windows.

1. **Create the pipes.**
   - Up-to-date Windows 11: open *Windows MIDI Services* and create "loopback" ports.
   - Otherwise: install [loopMIDI](https://www.tobias-erichsen.de/software/loopmidi.html) (free),
     then create these 4 ports with the **+** button (exact names):
     - `MIDIPadRouter A in`
     - `MIDIPadRouter A out`
     - `MIDIPadRouter B in`
     - `MIDIPadRouter B out`
2. **Plug in the Launchpad Mini** and start MIDIPadRouter. The wizard checks the Launchpad, then
   the 4 ports (🟢 found / 🔴 missing).
3. **In radio program A**: as MIDI *input* (or "controller"), choose `MIDIPadRouter A in`; as MIDI
   *output* (lights), choose `MIDIPadRouter A out`.
   **Do not select the Launchpad directly anymore**: MIDIPadRouter uses it.
4. **In radio program B**: same with `MIDIPadRouter B in` / `MIDIPadRouter B out`.
5. Choose the mode (Switch or Combined). That's it: the configuration is saved automatically.

If the header shows "Launchpad used by another program", one of your radio programs still
opens the Launchpad directly: change its setting (step 3) and restart it.

On macOS / Linux, the wizard offers "Create virtual ports": nothing to install.

## Day to day

- Closing the window **minimizes it to the notification area**: routing keeps running. The icon
  lets you change the active application, switch profile, or quit.
- **Profiles** (top): "Morning show", "Jingles"… one click to switch. Import/export as a `.json`
  file.
- **Settings** (⚙): language, theme, launch at startup, start minimized, switch key, rerun the
  wizard.
- **MIDI monitor** (bottom, collapsed): the last 50 messages, for troubleshooting.

The configuration is stored in `%APPDATA%\fr.midipadrouter.app\config.json`.

## Development

Requirements: stable Rust, Node 22, pnpm 10 (and the Visual Studio C++ build tools on Windows).

```bash
pnpm install
pnpm tauri dev      # run in development
pnpm test           # front-end tests (Vitest)
pnpm lint && pnpm check
cd src-tauri && cargo test && cargo clippy --all-targets -- -D warnings
pnpm tauri build    # MSI / NSIS installers in src-tauri/target/release/bundle
```

Layout: `src-tauri/src/devices` (Launchpad protocol), `src-tauri/src/midi` (router and ports),
`src/lib` (Svelte UI). See `DECISIONS.md` and `PERF.md`.
