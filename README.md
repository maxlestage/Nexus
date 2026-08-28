# Nexus One — manette une main (LEGO Technic + ESP32, 100 % Rust)

Manette de jeu **utilisable d'une seule main** (conçue pour une
hémiplégie droite, usage main gauche), compatible **Nintendo Switch**
(sans fil, émulation Pro Controller) et **PC/Mac** (HID Bluetooth), avec
une **app iPhone** pour remapper les boutons. Coque en **LEGO Technic**
pour pouvoir ajuster l'ergonomie à la personne, sans impression 3D.

```
┌─────────────────────────┐   BT Classic (HID)   ┌──────────┐
│  Manette « Nexus One »  │◄────────────────────►│  Switch  │
│  ESP32 + LEGO Technic   │   BLE HID            ├──────────┤
│                         │◄────────────────────►│  PC/Mac  │
│  firmware/  (Rust)      │   BLE GATT (config)  ├──────────┤
│  ▲ controller-core      │◄────────────────────►│  iPhone  │
└──┴──────────────────────┘                      │ app-ios/ │
                                                 └──────────┘
```

## Fonctionnalités

- ✅ **Une main** : joystick + 4 boutons sous le pouce, 4 gâchettes sous
  l'index/majeur, bouton de paume, couche **SHIFT** (croix + stick droit
  sur les mêmes commandes)
- ✅ **Switch** : émulation Pro Controller en Bluetooth classique
  (subcommands, calibration, rumble, numéro de joueur)
- ✅ **PC/macOS** : gamepad HID BLE natif (ZL maintenu à l'allumage)
- ✅ **App iPhone en Rust** (Dioxus) : remapping des 2 couches, turbo,
  macros, LEDs, vibrations, stats, batterie, OTA
- ✅ **Retour haptique** : DRV2605L — rumble de la console + clics locaux
- ✅ **RGB** : 8 × WS2812B (fixe, respiration, arc-en-ciel, réaction aux
  appuis) + états (appairage, joueur, batterie faible, OTA)
- ✅ **Turbo** : bouton TURBO physique (TURBO + bouton = rafale 1–30 Hz)
- ✅ **Macros** : accords de boutons → séquence (ex. A+B = X)
- ✅ **OTA** : mise à jour du firmware par WiFi depuis l'app
- ✅ **Stats** : compteur d'appuis par bouton, temps de jeu, macros
- ✅ **Batterie** 18650 + charge USB-C (TP4056), jauge remontée à la
  console et à l'app

## Arborescence

| Dossier | Contenu |
|---------|---------|
| [`controller-core/`](controller-core) | Logique partagée `no_std` : mapping, turbo, macros, stats, protocole Pro Controller, protocole de config — **testée sur PC** (`cargo test`) |
| [`firmware/`](firmware) | Firmware ESP32 (Rust + ESP-IDF) : Bluetooth, HID, haptique, LEDs, NVS, OTA |
| [`app-ios/`](app-ios) | App iPhone Dioxus + CoreBluetooth |
| [`web/`](web) | Site mobile-first (React 19 + TypeScript 7, compilé et servi par Bun), déployable sur Heroku |
| [`docs/BOM.md`](docs/BOM.md) | Liste d'achat (~60–90 €) |
| [`docs/WIRING.md`](docs/WIRING.md) | Câblage et brochage complet |
| [`docs/LEGO_BUILD.md`](docs/LEGO_BUILD.md) | Guide de construction LEGO Technic et ajustements ergonomiques |
| [`docs/BATTERY.md`](docs/BATTERY.md) | Alimentation, charge, sécurité |
| [`docs/PAIRING.md`](docs/PAIRING.md) | Appairage Switch/PC, app, raccourcis |

## Démarrage rapide

```bash
# Tester la logique (aucun matériel requis)
cargo test -p controller-core

# Flasher le firmware (voir firmware/README.md pour la toolchain Xtensa)
cd firmware && cargo run --release

# App iPhone (voir app-ios/README.md)
cd app-ios && dx build --platform ios --release

# Site (Bun uniquement — voir web/README.md)
bun install && bun run build && bun run start
```

## Déploiement du site

Le dépôt est déployable tel quel sur Heroku. Une seule commande n'est pas
devinable et doit précéder le premier push — le buildpack Node officiel ne
gère pas Bun :

```bash
heroku buildpacks:set https://github.com/jakeg/heroku-buildpack-bun -a VOTRE-APP
git push heroku master
```

Détails et dépannage : [web/DEPLOY.md](web/DEPLOY.md).

## Matériel requis (résumé)

ESP32-WROOM-32 (l'original, pas S3 — Bluetooth Classic obligatoire pour
la Switch), joystick analogique, 14 boutons, DRV2605L + vibreur,
bandeau WS2812B, TP4056 USB-C, accu 18650, MT3608, LEGO Technic.
Détails dans [docs/BOM.md](docs/BOM.md).

## Avertissement

Projet DIY : `controller-core` est testé, mais les modules Bluetooth du
firmware doivent être validés sur carte réelle (voir
`firmware/README.md`). Le protocole Pro Controller provient du reverse
engineering communautaire ([dekuNukem/Nintendo_Switch_Reverse_Engineering](https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering)).
