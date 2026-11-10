# MIDIPadRouter

**Un seul Launchpad Mini, plusieurs logiciels.** MIDIPadRouter (MIDIPR) se place entre votre
Novation Launchpad Mini MK1 et vos logiciels (par exemple deux logiciels de radio). Chaque logiciel
croit avoir « son » Launchpad.

- **Mode Bascule** : un logiciel à la fois reçoit tout le Launchpad. On change avec la touche ⇄ du
  Launchpad (bouton rond en haut à droite par défaut), un raccourci clavier (`Ctrl+Alt+1`,
  `Ctrl+Alt+2`…) ou un clic. Le Launchpad réaffiche aussitôt les lumières du logiciel choisi.
- **Mode Combiné** : tous les logiciels en même temps ; vous « peignez » chaque touche de la couleur
  du logiciel qui la reçoit.

Application légère (Tauri + Rust) : environ 0 % de processeur au repos.

## Installation

Téléchargez l'installeur Windows (`MIDIPadRouter_x.y.z_x64-setup.exe` ou `.msi`) et lancez-le.
macOS et Linux : compilez depuis les sources (voir plus bas).

## Comment brancher mes 2 logiciels de radio (Windows)

Chaque logiciel a besoin de **deux « tuyaux » MIDI** : un pour recevoir les pads, un pour renvoyer
les lumières. MIDIPadRouter ne peut pas créer ces tuyaux lui-même sous Windows.

1. **Créez les tuyaux.**
   - Windows 11 à jour : ouvrez *Windows MIDI Services* et créez des ports « loopback ».
   - Sinon : installez [loopMIDI](https://www.tobias-erichsen.de/software/loopmidi.html) (gratuit),
     puis créez, avec le bouton **+**, ces 4 ports (noms exacts) :
     - `MIDIPadRouter A in`
     - `MIDIPadRouter A out`
     - `MIDIPadRouter B in`
     - `MIDIPadRouter B out`
2. **Branchez le Launchpad Mini** et lancez MIDIPadRouter. L'assistant vérifie le Launchpad puis les
   4 ports (🟢 trouvé / 🔴 manquant).
3. **Dans le logiciel de radio A** : comme *entrée* MIDI (ou « contrôleur »), choisissez
   `MIDIPadRouter A in` ; comme *sortie* MIDI (lumières), choisissez `MIDIPadRouter A out`.
   **Ne choisissez plus le Launchpad directement** : c'est MIDIPadRouter qui l'utilise.
4. **Dans le logiciel de radio B** : pareil avec `MIDIPadRouter B in` / `MIDIPadRouter B out`.
5. Choisissez le mode (Bascule ou Combiné). C'est tout : la configuration est enregistrée
   automatiquement.

Si l'en-tête affiche « Launchpad occupé par un autre logiciel », un de vos logiciels de radio ouvre
encore le Launchpad directement : changez son réglage (étape 3) et redémarrez-le.

Sur macOS / Linux, l'assistant propose « Créer les ports virtuels » : rien à installer.

## Au quotidien

- Fermer la fenêtre la **réduit dans la zone de notification** : le routage continue. L'icône
  permet de changer de logiciel actif, de profil, ou de quitter.
- **Profils** (en haut) : « Émission matin », « Jingles »… un clic pour changer. Import/export en
  fichier `.json`.
- **Paramètres** (⚙) : langue, thème, lancement au démarrage, démarrage réduit, touche de bascule,
  relancer l'assistant.
- **Moniteur MIDI** (en bas, replié) : les 50 derniers messages, pour le dépannage.

La configuration est dans `%APPDATA%\fr.midipadrouter.app\config.json`.

## Développement

Prérequis : Rust stable, Node 22, pnpm 10 (et les outils de build C++ de Visual Studio sous
Windows).

```bash
pnpm install
pnpm tauri dev      # lancer en développement
pnpm test           # tests front (Vitest)
pnpm lint && pnpm check
cd src-tauri && cargo test && cargo clippy --all-targets -- -D warnings
pnpm tauri build    # installeurs MSI / NSIS dans src-tauri/target/release/bundle
```

Organisation : `src-tauri/src/devices` (protocole du Launchpad), `src-tauri/src/midi` (routeur et
ports), `src/lib` (interface Svelte). Voir `DECISIONS.md` et `PERF.md`.
