# MIDIPadRouter (MIDIPR) — Spécification pour Claude Code

> Colle ce fichier à la racine d'un dossier vide (ou renomme-le `CLAUDE.md`) puis lance Claude Code avec : « Lis SPEC et construis le projet étape par étape en suivant la section Jalons. Après chaque jalon, lance les tests et montre-moi le résultat. »

## 1. Objectif

**MIDIPadRouter** (abrégé **MIDIPR**) est une application de bureau **ultra légère** qui permet d'utiliser **un seul Novation Launchpad Mini MK1** avec **plusieurs logiciels en même temps ou en alternance** (ex. 2 logiciels de radio), en jouant le rôle de routeur/filtre MIDI entre le Launchpad physique et des ports MIDI virtuels que les logiciels cibles utilisent comme s'il s'agissait de « leur » Launchpad.

Public : utilisateur non technique (animateur radio). L'interface doit être évidente, sans manuel.

## 2. Périmètre

- Matériel supporté pour la v1 : **uniquement Launchpad Mini MK1** (Novation, USB class-compliant, 8×8 pads + 8 boutons du haut + 8 boutons latéraux). Le code doit isoler tout ce qui est spécifique au modèle derrière une interface `DeviceProfile` pour ajouter d'autres modèles plus tard, mais **ne pas implémenter d'autres modèles maintenant**.
- Plateforme prioritaire : **Windows 11**. macOS et Linux doivent compiler et fonctionner (même base de code), mais Windows est la référence de test.
- Nombre de logiciels cibles : **de 1 à 5**.

## 3. Fonctionnalités

### 3.1 Deux modes (sélecteur en haut de la fenêtre)

**Mode « Bascule » (Switch)**
- Un seul logiciel cible est actif à la fois ; il reçoit **tous** les boutons du Launchpad et lui envoie ses LED.
- Changement de cible actif par : clic dans l'UI, **raccourci clavier global** configurable (ex. `Ctrl+Alt+1..5`), et **un bouton du Launchpad dédié** (par défaut : bouton du haut n°8, configurable, réservé : jamais transmis aux logiciels).
- Au changement de cible : effacer le Launchpad puis **rejouer l'état LED mémorisé** de la nouvelle cible (voir 3.4), pour que le Launchpad reflète immédiatement son état.

**Mode « Combiné » (Combined)**
- Tous les logiciels sont actifs simultanément.
- Chaque touche du Launchpad (64 pads + 8 boutons du haut + 8 boutons latéraux = **80 touches**) est **attribuée à un seul logiciel** (ou « aucun »).
- Un événement sur une touche n'est envoyé qu'au logiciel propriétaire de cette touche.
- Les LED envoyées par un logiciel ne sont transmises au Launchpad que pour les touches qu'il possède.

### 3.2 Attribution touche par touche (éditeur visuel)
- Grille interactive reproduisant le Launchpad Mini MK1 (8×8 + rangée du haut + colonne de droite).
- Chaque logiciel cible a **une couleur** ; chaque touche affiche la couleur de son propriétaire.
- Interactions : clic = attribue la touche au logiciel « pinceau » sélectionné ; glisser = peindre plusieurs touches ; clic droit = « aucun » ; boutons d'aide : « Toute la grille → logiciel X », « Moitié gauche / droite », « Par colonne / par ligne », « Tout effacer ».
- Option « **Refléter sur le Launchpad** » : le Launchpad physique affiche brièvement (ou en continu quand aucun logiciel n'écrit dessus) la couleur de chaque propriétaire, pour visualiser le découpage.
- Annuler / rétablir (Ctrl+Z / Ctrl+Y).

### 3.3 Gestion des logiciels cibles (1 à 5)
Pour chaque cible : `nom` (libre, ex. « Radio A »), `couleur`, `port d'envoi vers le logiciel` (reçoit les pads), `port de réception depuis le logiciel` (LED), activé/désactivé.
- Les ports sont choisis dans une liste des ports MIDI détectés (rafraîchie à chaud, voir 3.6).
- **macOS/Linux** : bouton « Créer les ports virtuels » (via `midir` virtual ports).
- **Windows** : `midir` ne peut pas créer de ports virtuels. L'app affiche un **assistant** clair : « Windows 11 à jour : utilise le loopback intégré de Windows MIDI Services, sinon installe loopMIDI et crée N paires de ports nommées `MIDIPadRouter A in/out`, etc. » avec lien et vérification automatique que les ports existent. L'application doit fonctionner avec des ports créés par loopMIDI ou par le loopback Windows MIDI Services — elle se contente de les ouvrir.
- Rappel dans l'assistant : un loopback est un « tuyau » — il faut **deux ports par cible** (un par sens) pour éviter les boucles.

### 3.4 Retour LED et mémoire d'état
- L'app maintient un **tampon d'état LED par cible** (80 entrées : vélocité/couleur courante) mis à jour à chaque message reçu du logiciel, même quand la cible n'est pas visible sur le Launchpad (mode bascule).
- En bascule : au changement, reset du Launchpad puis envoi du tampon de la nouvelle cible.
- En combiné : filtre par propriétaire ; un message LED d'une touche non possédée est ignoré (mais gardé en tampon).
- Gérer aussi : message de reset (`B0 00 00`), changement de mode de layout/buffer, qui doivent être filtrés ou interprétés proprement (ne jamais laisser un logiciel mettre le Launchpad en mode « drum rack » ou changer le buffer sans que l'état interne soit cohérent — le plus simple : **bloquer** ces messages spéciaux et émuler leur effet sur le tampon).

### 3.5 Profils / persistance
- Configuration en JSON dans le dossier de config de l'OS (`dirs`/Tauri path API).
- **Profils nommés** (ex. « Émission matin », « Jingles ») : mode, cibles, attribution, raccourcis. Changement de profil en un clic.
- Import/export de profil en fichier `.json`.
- Sauvegarde automatique, écriture atomique (fichier temporaire + rename).

### 3.6 Robustesse
- Détection du **branchement/débranchement** du Launchpad (scrutation légère toutes les ~2 s uniquement quand le Launchpad est absent, pas de scrutation sinon) et reconnexion automatique avec ré-affichage de l'état.
- Idem pour les ports des cibles : état par cible (🟢 connecté / 🔴 port introuvable) dans l'UI.
- Aucune panique/crash si un port disparaît : la cible passe en erreur, le reste continue.
- **Aucune boucle de feedback** : ne jamais router une sortie vers une entrée du même groupe ; valider la config au chargement.
- Si le nom USB du Launchpad contient « Launchpad Mini » mais pas MK1 exact (MK2/MK3 utilisent d'autres noms/protocoles), afficher un avertissement « Modèle non supporté » plutôt que de mal fonctionner.

### 3.7 Divers
- Icône dans la zone de notification : afficher la cible active, changer de cible/profil, quitter.
- Option « Lancer au démarrage de Windows » et « Démarrer réduit ».
- Moniteur MIDI minimal (panneau repliable) : dernières ~50 trames avec direction et cible, pour le dépannage. **Désactivé par défaut** et n'envoie rien au front-end tant qu'il est replié.
- Langues : **français par défaut**, anglais en second (fichiers i18n simples).

## 4. Protocole Launchpad Mini MK1 (à respecter)

Référence : Novation « Launchpad Programmer's Reference » (MK1). Vérifier ces points dans la doc avant d'implémenter et les mettre en constantes dans `devices/launchpad_mini_mk1.rs` avec tests.

- **Pads 8×8** (layout X-Y par défaut) : Note On/Off canal 1 ; `note = 16 * ligne + colonne` (ligne 0 = haut, colonne 0 = gauche). Appui = Note On vélocité 127, relâchement = Note Off (ou Note On vélocité 0).
- **Boutons latéraux (scène, colonne de droite)** : notes `8, 24, 40, 56, 72, 88, 104, 120` (colonne 8 de chaque ligne).
- **Boutons du haut** : Control Change `104..111` (0x68..0x6F), valeur 127 = appui, 0 = relâchement, canal 1.
- **LED** : on envoie Note On sur la même note avec vélocité = `16 * vert + rouge + flags` (vert et rouge de 0 à 3, flags = 12 pour affichage normal, 8 pour « copie » en double buffering, 0 pour clignotement selon le mode). Pour les boutons du haut : CC correspondant avec la même valeur.
- **Reset** : `B0 00 00`.
- Utiliser une table `Key` (enum / index 0..79) avec conversions `Key <-> (type, numéro MIDI)` testées par des tests unitaires exhaustifs (aller-retour sur les 80 touches).

## 5. Stack technique (moderne et légère)

| Couche | Choix | Pourquoi |
|---|---|---|
| Shell desktop | **Tauri 2** | Binaire de quelques Mo, ~30–60 Mo de RAM, utilise la WebView du système, pas de Chromium embarqué |
| Cœur / MIDI | **Rust** + crate **`midir`** | Faible latence, sûr, pas de GC |
| Front-end | **Svelte 5 (runes) + TypeScript + Vite** | Très léger à l'exécution, réactif, pas de virtual DOM |
| Style | **Tailwind CSS 4** + variables CSS (thème clair/sombre auto) | Rapide à écrire, rendu moderne |
| Raccourcis globaux | plugin Tauri `global-shortcut` | |
| Tray / autostart | API tray Tauri + plugin `autostart` | |
| Persistance | `serde` + `serde_json` | |
| Tests | `cargo test` (cœur), **Vitest** (front), tests d'intégration MIDI avec ports factices | |

Gestionnaire de paquets : `pnpm`. Pas de bibliothèque d'UI lourde (pas de framework de composants géants) : composants maison simples.

## 6. Architecture

```
src-tauri/
  src/
    main.rs               // bootstrap Tauri, tray, shortcuts
    app_state.rs          // état partagé (Arc<...>), config courante
    config.rs             // modèles serde + validation + migrations de version
    devices/
      mod.rs              // trait DeviceProfile (touches, parsing, rendu LED)
      launchpad_mini_mk1.rs
    midi/
      ports.rs            // énumération, ouverture, hot-plug léger
      router.rs           // cœur: routage pad->cible, LED->launchpad
      led_state.rs        // tampons LED par cible
      messages.rs         // parse/encode MIDI (typé, sans allocation sur le chemin chaud)
    commands.rs           // commandes Tauri exposées au front
    events.rs             // événements envoyés au front (throttlés)
src/                      // front Svelte
  lib/ (stores, i18n, components: PadGrid, TargetList, ModeSwitch, ProfileMenu, MidiMonitor, SetupWizard)
```

### Règles de performance (impératives)
1. **Le chemin chaud MIDI ne passe jamais par le front-end** : le routage s'exécute entièrement en Rust, dans les callbacks `midir` (pas d'allocation, pas de lock bloquant long ; utiliser `ArcSwap`/`parking_lot` pour lire la config d'attribution, une table `[Option<TargetId>; 80]` précalculée).
2. Le front-end n'est **notifié que sur changement** d'état utile (cible active, connexion/déconnexion, activité par cible pour un petit témoin lumineux), avec **throttling ≤ 30 Hz**, et **rien n'est envoyé quand la fenêtre est masquée/réduite**.
3. **Aucune boucle de rendu continue** (pas de `requestAnimationFrame` permanent, pas d'animations infinies) ; animations CSS courtes uniquement.
4. Pas de scrutation permanente : scrutation des ports uniquement si un port attendu est absent, avec backoff (2 s → 5 s).
5. Objectifs mesurables à vérifier à la fin : **CPU ≈ 0 % au repos**, < 1 % en usage intensif, **RAM < 80 Mo**, latence ajoutée par le routage **< 1 ms** (mesurée en test), binaire installé < 15 Mo.

## 7. Modèle de données (config JSON, versionné)

```jsonc
{
  "version": 1,
  "activeProfile": "default",
  "profiles": {
    "default": {
      "mode": "switch",              // "switch" | "combined"
      "activeTarget": "a",           // utilisé en mode switch
      "switchKey": { "type": "cc", "number": 111 }, // touche de bascule réservée (null = aucune)
      "shortcuts": { "a": "Ctrl+Alt+1", "b": "Ctrl+Alt+2" },
      "targets": [
        { "id": "a", "name": "Radio A", "color": "#3b82f6", "enabled": true,
          "portToApp": "MIDIPadRouter A in", "portFromApp": "MIDIPadRouter A out" }
      ],
      "assignment": { "pad:0": "a", "pad:1": "b", "cc:104": "a" } // clé = touche, valeur = id de cible ; absent = aucune
    }
  },
  "settings": { "language": "fr", "autostart": false, "startMinimized": false, "theme": "auto" }
}
```
Validation : max 5 cibles, ids uniques, pas de port utilisé par deux cibles, pas d'attribution vers une cible inexistante, `switchKey` non attribuable à une cible.

## 8. Interface (principes)

- Une seule fenêtre (~900×600, redimensionnable), thème clair/sombre selon l'OS, coins arrondis, typographie système, espacement généreux, contrastes AA.
- Barre du haut : **sélecteur de mode** (2 grands segments « Bascule » / « Combiné »), sélecteur de profil, état du Launchpad (🟢/🔴).
- Zone centrale gauche : **grille Launchpad interactive** (SVG/CSS grid, pas de canvas). En mode bascule : la grille montre le Launchpad avec l'état LED de la cible active ; en mode combiné : l'éditeur d'attribution coloré.
- Zone droite : **liste des cibles** (cartes avec nom, couleur, état des ports, témoin d'activité, bouton « Actif » en mode bascule, bouton « + Ajouter » jusqu'à 5).
- Assistant de premier lancement en 3 étapes : (1) détecter le Launchpad Mini MK1, (2) créer/choisir les ports des logiciels, (3) choisir le mode. Doit pouvoir être relancé depuis les paramètres.
- Messages d'erreur en langage clair (« Le logiciel Radio B n'écoute pas ce port » etc.), jamais de jargon brut.
- Accessible au clavier, libellés ARIA sur la grille.

## 9. Tests et critères d'acceptation

**Tests unitaires (Rust)** : conversion des 80 touches ↔ messages MIDI ; encodage des vélocités LED ; filtrage par propriétaire ; mémoire/rejeu LED au changement de cible ; validation de config ; blocage des messages spéciaux.
**Tests d'intégration** : routeur avec ports MIDI factices (trait `MidiIn`/`MidiOut` mockables) — simuler pad pressé en bascule/combiné, LED en retour, débranchement d'une cible.
**Tests front (Vitest)** : stores, peinture d'attribution, annuler/rétablir, i18n.

Critères d'acceptation :
1. En mode combiné, 2 cibles : une touche attribuée à A n'arrive **jamais** à B, et inversement.
2. En mode bascule : après changement de cible, le Launchpad affiche l'état de la nouvelle cible en < 100 ms.
3. Les cibles continuent de fonctionner si l'une d'elles perd son port.
4. Redémarrage de l'app : config et profil retrouvés à l'identique.
5. Objectifs de performance de la section 6 respectés (mesures consignées dans `PERF.md`).

## 10. Jalons (travailler dans cet ordre, committer à chaque jalon)

1. **Squelette** : projet Tauri 2 + Svelte 5 + TS + Tailwind, lint/format (clippy, rustfmt, eslint, prettier), CI GitHub Actions (build + tests sur Windows).
2. **Cœur MIDI** : `DeviceProfile` + Launchpad Mini MK1, énumération/ouverture de ports, journal console de ce que reçoit le Launchpad. Tests unitaires du protocole.
3. **Routeur mode bascule** + tampon LED + rejeu au changement + clavier/touche dédiée.
4. **Routeur mode combiné** + table d'attribution + filtrage LED par propriétaire.
5. **Config/profils/persistance** + validation + migrations.
6. **UI** : grille interactive, éditeur de peinture, liste des cibles, sélecteur de mode, profils.
7. **Assistant de configuration des ports** (Windows : détection loopMIDI / loopback Windows MIDI Services ; macOS/Linux : ports virtuels).
8. **Tray, raccourcis globaux, autostart, moniteur MIDI**, hot-plug.
9. **i18n FR/EN, accessibilité, finitions visuelles**, icône d'application.
10. **Mesures de performance** (`PERF.md`), packaging (installeur MSI/NSIS Windows), README en français avec captures et guide « comment brancher mes 2 logiciels de radio ».

## 11. Hors périmètre (v1)
Autres modèles de Launchpad (MK2/MK3/Pro/X), transformation/remappage de notes, canaux MIDI multiples, scripts utilisateur, synchronisation cloud, création de ports virtuels sous Windows (dépend de loopMIDI ou du loopback Windows MIDI Services).

## 12. Consignes à Claude Code
- Avant de coder le protocole Launchpad, **vérifie la doc officielle MK1** (WebSearch/WebFetch) et corrige la section 4 si besoin.
- Si une décision n'est pas couverte ici, choisis la plus simple et la plus légère, note-la dans `DECISIONS.md`.
- Pas de dépendances superflues ; justifie chaque nouvelle crate/paquet.
- Écris du code commenté en anglais, une UI en français (i18n).
- Ne fais jamais passer le trafic MIDI par le front-end.
