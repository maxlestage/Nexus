# Appairage et utilisation

## Nintendo Switch (mode par défaut)

1. Allumer la manette : la LED 0 clignote en **bleu** (recherche).
2. Console : **Manettes → Changer le style/l'ordre**.
3. « Pro Controller » apparaît dans la liste ; sélectionner.
4. LED fixe + LEDs 1..4 = numéro de joueur : c'est prêt.
5. Les fois suivantes, la manette se reconnecte seule (appuyer sur
   n'importe quel bouton).

Dépannage :
- La console ne voit rien → éteindre/rallumer la manette, rester sur
  l'écran « Changer le style/l'ordre » (c'est le seul écran où la
  console accepte de nouvelles manettes).
- Connexion puis déconnexion immédiate → oublier la manette
  (Manettes → Désynchroniser) et recommencer.
- Le rumble HD de la Switch est converti pour le DRV2605 : normal qu'il
  soit moins nuancé qu'un vrai Pro Controller.

## PC / Mac (mode HID BLE)

1. Allumer la manette **en maintenant la gâchette majeur-bas** (ZL).
2. Windows : Paramètres → Bluetooth → Ajouter un appareil →
   « Nexus One ». macOS : Réglages → Bluetooth.
3. La manette apparaît comme gamepad 16 boutons + 4 axes (testable sur
   gamepad-tester.com, Steam, etc.).

## App iPhone

1. Ouvrir l'app « Nexus One » (voir `app-ios/README.md`).
2. « Se connecter à la manette » — fonctionne dans les deux modes, même
   pendant une partie (BLE et BT classique coexistent).
3. Onglets : Boutons (remapping normal/SHIFT), Turbo, Macros, Stats,
   Réglages (LEDs, vibrations, batterie, OTA, réglages d'usine).
4. « Enregistrer sur la manette » persiste la config en flash.

## Raccourcis sur la manette

- **TURBO + bouton** : active/désactive la rafale sur ce bouton
  (double clic haptique = ON, buzz = OFF).
- **SHIFT maintenu** : les 4 boutons du pouce deviennent la croix, le
  joystick devient le stick droit (caméra) — configurable dans l'app.
- **ZL maintenu à l'allumage** : mode PC.
