# App iPhone — Nexus One (100 % Rust)

Application de configuration de la manette : remapping des boutons,
turbo, macros, LEDs, vibrations, statistiques, batterie et mise à jour
OTA. Construite avec [Dioxus](https://dioxuslabs.com) (UI) et
[btleplug](https://github.com/deviceplug/btleplug) (CoreBluetooth).

## Prérequis

- macOS avec Xcode (signature iOS) et un compte développeur Apple
  (un compte gratuit suffit pour installer sur son propre iPhone).
- `cargo install dioxus-cli`
- `rustup target add aarch64-apple-ios`

## Lancer sur iPhone

```bash
cd app-ios
dx build --platform ios --release
dx serve --platform ios          # simulateur (sans Bluetooth !)
```

Le Bluetooth ne fonctionne pas dans le simulateur iOS : tester sur un
appareil réel. Dans le projet Xcode généré, vérifier que `Info.plist`
contient :

```xml
<key>NSBluetoothAlwaysUsageDescription</key>
<string>Connexion à la manette Nexus One pour la configurer.</string>
```

## Fonctionnement

L'app parle au service BLE `6e400001-c352-…` de la manette (voir
`controller-core/src/protocol.rs`) : chaque écran encode des messages
`Request`/`Response` en postcard — exactement le même code Rust que le
firmware, donc pas de dérive de protocole possible.
