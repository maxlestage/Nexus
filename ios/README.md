# App iPhone — Nexus One (Swift natif)

Application de configuration de la manette : remappage des deux couches,
turbo, macros, statistiques, éclairage, vibrations, batterie et mise à jour
du firmware. SwiftUI + CoreBluetooth.

## Pourquoi Swift, et pourquoi quand même du Rust

L'interface est en Swift natif. Mais le **protocole reste en Rust** : le
crate `ios-bridge/` compile `controller-core` — le code embarqué dans le
firmware — en bibliothèque statique, et l'expose derrière une petite API C.

L'application ne réimplémente donc pas l'encodage `postcard` : elle manipule
du JSON et laisse Rust produire les octets envoyés en Bluetooth. Firmware et
application ne peuvent pas diverger, ce qui serait la panne la plus pénible
à diagnostiquer.

```
SwiftUI  ──JSON──►  ios-bridge (Rust)  ──postcard──►  BLE  ──►  manette
```

## Construire

Le `.xcodeproj` n'est pas versionné, il se génère :

```bash
brew install xcodegen
cd ios
DEVELOPMENT_TEAM=VOTRE_TEAM_ID xcodegen generate
open NexusOne.xcodeproj
```

La compilation du pont Rust est automatique (script exécuté avant chaque
build). Depuis la ligne de commande :

```bash
fastlane check    # compile sans signer
fastlane beta     # compile, signe et envoie sur TestFlight
```

## Publication automatique

Le workflow `.github/workflows/ios.yml` compile et envoie sur TestFlight à
chaque push sur `master`. Il attend quatre secrets GitHub :

| Secret | Contenu |
|---|---|
| `ASC_ISSUER_ID` | Issuer ID de la clé App Store Connect |
| `ASC_KEY_ID` | Key ID de la clé |
| `ASC_KEY_P8` | contenu du fichier `.p8`, de `-----BEGIN` à `-----END` inclus |
| `APPLE_TEAM_ID` | identifiant de votre équipe Apple Developer |

Aucun certificat n'est stocké dans le dépôt : Xcode crée et récupère
lui-même certificat et profil grâce à la clé (`-allowProvisioningUpdates`).
La clé est écrite dans un fichier temporaire puis effacée, y compris si le
job échoue.

Sans secrets — sur une pull request, ou avant leur configuration — le
workflow se contente d'une compilation non signée : elle valide le Swift et
le pont sans toucher au compte Apple.

## Ce qui est vérifié, et ce qui ne l'est pas

Le pont Rust est testé (`cargo test -p nexus-bridge`), passe clippy en
`-D warnings` et compile pour `aarch64-apple-ios`. Le workflow rejoue ces
trois vérifications sur Linux avant même de réserver un runner macOS.

Le code Swift, lui, n'a jamais été compilé : cela demande un Mac. La
première exécution du workflow est donc le vrai test — attendez-vous
éventuellement à quelques erreurs de compilation à corriger.

Le Bluetooth ne fonctionne pas dans le simulateur : tester sur un iPhone.
