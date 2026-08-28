# Nomenclature (BOM)

Budget total indicatif : **60–90 €** (hors LEGO).

## Électronique

| Qté | Composant | Rôle | Prix indicatif |
|-----|-----------|------|----------------|
| 1 | **ESP32-WROOM-32 DevKitC** (ESP32 classique, PAS S2/S3/C3) | Cerveau ; seul l'ESP32 d'origine a le Bluetooth Classic exigé par la Switch | 6–10 € |
| 1 | Joystick analogique type PSP/Switch (2 axes + clic) ou module KY-023 | Stick principal | 3–5 € |
| 14 | Boutons tactiles 12 × 12 mm avec chapeaux (ou boutons arcade 24 mm pour les 4 du pouce) | Entrées | 5–10 € |
| 1 | **DRV2605L** (module Adafruit ou équivalent, I2C) | Pilote haptique | 8 € |
| 1 | Moteur vibreur ERM 3 V (pastille 10 mm) ou LRA | Retour haptique | 2 € |
| 1 | Bandeau **WS2812B** 8 LEDs (ou anneau) | Éclairage RGB | 3 € |
| 1 | **TP4056 USB-C** avec protection (modules à 6 broches OUT+/OUT−) | Charge Li-ion | 2 € |
| 1 | Accu Li-ion **18650** 2600–3500 mAh + support, OU LiPo 103450 2000 mAh | Batterie (~10–20 h d'autonomie) | 5–8 € |
| 1 | Convertisseur élévateur **MT3608** (ou module 5 V « powerbank ») | 3,7 V → 5 V pour l'ESP32 et les LEDs | 2 € |
| 1 | Interrupteur à glissière | Marche/arrêt | 1 € |
| 2 | Résistances 100 kΩ | Pont diviseur mesure batterie | — |
| 1 | Résistance 10 kΩ | Pull-up externe GPIO39 (bouton SHIFT) | — |
| 1 | Résistance 330 Ω + condensateur 1000 µF | Protection ligne WS2812B | 1 € |
| — | Fils AWG26 souples, gaine thermo, veroboard ou PCB proto | Câblage | 3 € |

## LEGO Technic (voir docs/LEGO_BUILD.md)

Un set « chassis » d'occasion suffit ; pièces clés :

- Poutres 1×11 / 1×15 percées (structure), poutres coudées 3×5 (poignée)
- Plaques Technic 5×11 (support carte)
- Axes 3/4/5 tenons, connecteurs et goupilles (friction)
- 2 engrenages 12 dents + crémaillère (optionnel : gâchettes à ressort)
- Élastiques LEGO (rappel des gâchettes)

## Outils

Fer à souder, multimètre, pistolet à colle (fixation des modules sur des
plaques LEGO percées), petit tournevis.
