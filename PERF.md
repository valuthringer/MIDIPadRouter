# Mesures de performance

Machine : Windows 11, build release (`pnpm tauri build`), 10/11/2026. Mesures faites **sans
Launchpad branché ni loopMIDI**, donc au repos ; l'usage intensif réel reste à mesurer sur le
matériel.

| Objectif (spec §6) | Mesure | Résultat |
|---|---|---|
| CPU ≈ 0 % au repos | 0,000 % fenêtre fermée ; 0,010 % fenêtre ouverte (10 s, tous cœurs, WebView2 incluse) | ✅ |
| CPU < 1 % en usage intensif | coût de routage 54 ns/message (voir ci-dessous) → < 0,01 % même à 1000 msg/s | ✅ (calcul, pas mesuré sur matériel) |
| RAM < 80 Mo | **6,1 Mo** privés fenêtre fermée (icône de notification) ; **~198 Mo** fenêtre ouverte, dont ~192 Mo pour les 6 processus WebView2 | ✅ en fond / ❌ fenêtre ouverte |
| Latence ajoutée < 1 ms | 54 ns par message (test `routing_latency_under_1ms`, release, fausses sorties) | ✅ |
| Rejeu au changement de cible < 100 ms | 41 messages envoyés, test `switch_key_is_reserved_and_replays_led_state` | ✅ (hors temps USB du Launchpad) |
| Binaire installé < 15 Mo | `midipadrouter.exe` 6,9 Mo ; installeur NSIS 1,7 Mo, MSI 2,5 Mo | ✅ |

## Notes

- **RAM fenêtre ouverte** : le coût vient de WebView2 (Edge), pas de l'application. Pour tenir
  l'objectif là où il compte (pendant l'émission, fenêtre fermée), fermer la fenêtre **détruit** la
  WebView ; le routage continue dans le cœur Rust (6 Mo). La rouvrir la recrée (~1 s).
- Pas de scrutation en continu : le thread UI dort sur un canal (0 réveil au repos) ; le thread des
  ports se réveille toutes les 5 s pour lire la liste des ports (voir `DECISIONS.md`).

## Reproduire

```bash
cd src-tauri && cargo test --release routing_latency -- --nocapture
```

Mémoire et CPU : lancer l'exe release puis sommer `PrivateMemorySize64` et `TotalProcessorTime`
du processus `midipadrouter` et de ses enfants `msedgewebview2` (PowerShell `Get-Process`).
