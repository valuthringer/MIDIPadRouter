# Décisions

Choix non couverts (ou ajustés) par la spécification, du plus important au moins important.

## Protocole Launchpad Mini MK1 (vérification de la section 4)

- Vérifié contre le *Launchpad Programmer's Reference* et l'implémentation de la crate `launchy`.
- **Correction de la spec** : les drapeaux LED sont `12` = affichage normal, `8` = **clignotement**
  (bit *copy* seul, utilisé avec `B0 00 28`), `0` = **double buffering** (écrit seulement le tampon
  de mise à jour). La spec inversait « copie » et « clignotement ».
- Constantes et tests dans `src-tauri/src/devices/launchpad_mini_mk1.rs`.
- L'ordre des index de touches (0..63 grille, 64..71 latéraux, 72..79 haut) est celui de la
  *mise à jour rapide* (`92h`) du manuel : le décodage des mises à jour rapides d'un logiciel
  devient trivial.
- Messages spéciaux envoyés par les logiciels :
  - `B0 00 00` (reset) : **émulé** sur le tampon du logiciel ; on éteint seulement ses touches visibles.
  - `B0 00 01/02` (layout X-Y / drum rack), contrôle de tampon autre que `20h`/`28h`, test LED
    `7Dh..7Fh`, duty cycle (CC 30/31), SysEx, autres canaux : **bloqués**.
  - `B0 00 20` / `B0 00 28` (clignotement automatique arrêt/marche, tampon 0) : transmis, sans
    effet sur la cohérence de l'état.
  - `92h` (mise à jour rapide) : décodé en LEDs individuelles, filtré par propriétaire.
- Rejeu au changement de cible : `B0 00 00` puis 40 messages de mise à jour rapide (80 LEDs).
- « Launchpad Mini » est accepté (MK1 et Mini MK2, même protocole). Tout autre nom contenant
  « Launchpad », « MK3 », « LPMini », « LPX », « LPPro » affiche « Modèle non supporté ».

## Architecture

- **Un seul `Mutex` std** autour du routeur au lieu de `ArcSwap`/`parking_lot` : le trafic MIDI est
  minuscule, le verrou n'est tenu que le temps de quelques `send` (mesuré < 1 µs par message, voir
  `PERF.md`) et cela évite deux dépendances. Les connexions d'entrée sont toujours fermées verrou
  relâché (leur fermeture attend le callback qui prend ce verrou).
- `led_state.rs` et `messages.rs` de l'arborescence proposée n'existent pas : le tampon LED est un
  `[u8; 80]` dans `Target` et l'analyse/encodage MIDI est par nature propre au modèle, donc dans le
  `DeviceProfile`. `tray.rs` est séparé de `main.rs` pour la lisibilité.
- Notifications UI : canal borné (`sync_channel(1)`, `try_send`, sans allocation) réveillant un
  thread qui attend 33 ms (≤ 30 Hz), puis émet un instantané complet seulement si la fenêtre est
  visible et non réduite. Au retour de la fenêtre, le front redemande l'état.
- `activeTarget` appartient au backend (touche du Launchpad, raccourci, tray) : il est persisté par le
  thread UI, hors du chemin chaud, et la sauvegarde venant du front ne l'écrase pas.

## Robustesse

- **Écart volontaire à la spec 3.6** : quand tout est connecté, la liste des ports est relue toutes
  les **5 s** (pas seulement quand un port manque). Sans cela, un Launchpad débranché puis
  rebranché sans trafic LED entre-temps reste muet pour toujours (midir ne signale pas les
  débranchements). Coût mesuré : négligeable. Quand un port manque : 2 s puis 5 s.
- Nouveau statut **« Launchpad occupé »** : le Launchpad est présent mais ne peut être ouvert (un
  logiciel de radio l'utilise encore directement — cas le plus fréquent sous Windows).
- Config illisible/invalide au démarrage : renommée en `config.invalid.json` (jamais supprimée),
  configuration par défaut chargée, message à l'utilisateur.
- Plugin `single-instance` : une 2ᵉ instance se battrait pour le Launchpad ; elle met la 1ʳᵉ au
  premier plan.
- Fermer la fenêtre **détruit la WebView** (≈ 190 Mo libérés, voir `PERF.md`) ; le routage continue
  dans le cœur Rust et l'icône de notification reste. « Quitter » est dans le menu de l'icône. La
  fenêtre est recréée à l'ouverture (icône, 2ᵉ lancement, sauf « Démarrer réduit »).
- Les raccourcis globaux ne sont enregistrés qu'en mode Bascule (ils ne servent qu'à changer de
  cible) : en mode Combiné les touches restent libres pour les autres programmes.
- Clés d'attribution : `pad:<note MIDI>` et `cc:<numéro>`, comme `switchKey` (`type` = `pad`|`cc`).

## Interface

- Le pinceau se choisit dans la barre d'outils de la grille ; « par colonne / par ligne » est une
  **forme de pinceau** (un clic peint la ligne/colonne entière).
- La touche de bascule est choisie parmi les 16 boutons ronds (haut et latéraux) ; elle n'est
  utilisée qu'en mode Bascule mais reste réservée dans les deux modes (validation).
- « Refléter sur le Launchpad » est un interrupteur : tant qu'il est actif, le Launchpad montre le
  propriétaire de chaque touche (couleurs Launchpad approximatives : vert, rouge, ambre, jaune,
  orange), les LEDs des logiciels restent mémorisées puis sont rejouées.
- En mode Bascule, la touche de bascule s'allume dans la couleur Launchpad de la cible active.
- Ports virtuels (macOS/Linux) : réglage global `virtualPorts` ; noms `MIDIPadRouter X in/out`.
- Liens d'aide ouverts par une commande Rust avec URL fixes (pas de permission « ouvrir n'importe
  quelle URL » côté web).

## Dépendances

Rust :
- `tauri` (+ `tray-icon`) : shell imposé par la spec.
- `tauri-plugin-global-shortcut`, `tauri-plugin-autostart` : imposés par la spec.
- `tauri-plugin-dialog` : sélection de fichier pour l'import/export de profils.
- `tauri-plugin-opener` : ouvrir les pages loopMIDI / Windows MIDI Services.
- `tauri-plugin-single-instance` : voir Robustesse.
- `midir`, `serde`, `serde_json` : imposés par la spec.

JS (dev seulement, sauf `@tauri-apps/api`) : Svelte 5, Vite, TypeScript, Tailwind 4, Vitest,
ESLint, Prettier, svelte-check — imposés par la spec. TypeScript est fixé en 6.0 car
`typescript-eslint` ne prend pas encore TypeScript 7 en charge.
