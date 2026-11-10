# Performance measurements

Machine: Windows 11, release build (`pnpm tauri build`), 2026-11-10. Measurements were taken
**without a Launchpad plugged in and without loopMIDI**, i.e. at idle; real heavy usage still has
to be measured on hardware.

| Target (spec §6) | Measurement | Result |
|---|---|---|
| CPU ≈ 0 % at idle | 0.000 % window closed; 0.010 % window open (10 s, all cores, WebView2 included) | ✅ |
| CPU < 1 % under heavy use | routing cost 54 ns/message (see below) → < 0.01 % even at 1000 msg/s | ✅ (computed, not measured on hardware) |
| RAM < 80 MB | **6.1 MB** private, window closed (tray icon); **~198 MB** window open, of which ~192 MB for the 6 WebView2 processes | ✅ in background / ❌ window open |
| Added latency < 1 ms | 54 ns per message (test `routing_latency_under_1ms`, release, fake outputs) | ✅ |
| Replay on target switch < 100 ms | 41 messages sent, test `switch_key_is_reserved_and_replays_led_state` | ✅ (excluding the Launchpad's USB time) |
| Installed binary < 15 MB | `midipadrouter.exe` 6.9 MB; NSIS installer 1.7 MB, MSI 2.5 MB | ✅ |

## Notes

- **RAM with the window open**: the cost comes from WebView2 (Edge), not from the application. To
  meet the target where it matters (during a broadcast, window closed), closing the window
  **destroys** the WebView; routing keeps running in the Rust core (6 MB). Reopening it recreates
  the WebView (~1 s).
- No continuous polling: the UI thread sleeps on a channel (0 wake-ups at idle); the ports thread
  wakes every 5 s to read the port list (see `DECISIONS.md`).

## Reproduce

```bash
cd src-tauri && cargo test --release routing_latency -- --nocapture
```

Memory and CPU: run the release exe, then sum `PrivateMemorySize64` and `TotalProcessorTime` for
the `midipadrouter` process and its `msedgewebview2` children (PowerShell `Get-Process`).
