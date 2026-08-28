# Firmware ESP32 — Nexus One

Firmware Rust (std, ESP-IDF) pour ESP32 classique (WROOM-32). L'ESP32
d'origine est **obligatoire** : c'est le seul de la famille avec du
Bluetooth Classic (BR/EDR), requis pour l'appairage sans fil avec la
Switch (les S2/S3/C3 sont BLE uniquement).

## Modes

| Mode | Déclenchement | Transport |
|------|---------------|-----------|
| **Switch** | démarrage normal | Bluetooth Classic, émulation Pro Controller (VID 0x057E / PID 0x2009) |
| **PC** | gâchette majeur-bas maintenue à l'allumage | BLE HID gamepad générique (Windows/macOS/Linux sans pilote) |

Dans les deux modes, le service BLE de configuration (app iPhone) est actif.

## Compilation

```bash
# 1. Toolchain Xtensa + outils (une seule fois)
cargo install espup espflash ldproxy
espup install && . $HOME/export-esp.sh

# 2. Compilation + flash + moniteur série
cd firmware
cargo run --release
```

La première compilation télécharge et compile ESP-IDF v5.2 (long). Les
options Bluetooth dual-mode sont dans `sdkconfig.defaults`, la table de
partitions OTA (2 slots) dans `partitions.csv`.

## Appairage Switch

1. Allumer la manette (LED 0 clignote en bleu).
2. Sur la console : **Manettes → Changer le style/l'ordre**.
3. « Pro Controller » apparaît ; la LED passe fixe une fois connecté et
   les LEDs 1..4 indiquent le numéro de joueur.

## Limites connues, à valider sur carte

- **Mode PC + service de configuration** : le HID BLE (`esp_hid`) et le
  service GATT de config enregistrent chacun des callbacks GATTS/GAP
  Bluedroid globaux ; ils peuvent se marcher dessus (l'un des deux devient
  sourd) et se partagent la même publicité BLE. À valider en mode PC ; au
  besoin, ne démarrer le service de config qu'en mode Switch.
- **Rapport court 0x3F** : l'ordre exact des bits boutons est une
  approximation raisonnable du format documenté ; à vérifier sur l'écran
  d'appairage de la console.
- **Longueurs de rapports** : le report map déclare les entrées vendeur à
  63 octets et le firmware envoie des payloads bourrés à cette taille pour
  satisfaire la vérification d'`esp_hid` ; à confirmer sur cible.

## État d'avancement

- `controller-core` (mapping, turbo, macros, stats, protocole Pro
  Controller, protocole de config) est **testé sur PC** (`cargo test`).
- Les modules Bluetooth (`src/bt/`) s'appuient sur les API Bluedroid /
  `esp_hid` d'ESP-IDF via `esp-idf-sys` ; ils doivent être validés sur
  carte réelle (les noms exacts des bindings peuvent bouger d'une version
  d'`esp-idf-sys` à l'autre — compiler et ajuster).
- L'appairage Switch nécessite parfois plusieurs tentatives le temps que
  la console mette en cache la manette ; voir `docs/PAIRING.md`.
