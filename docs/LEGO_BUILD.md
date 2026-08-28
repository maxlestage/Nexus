# Construction LEGO Technic — manette une main (main gauche)

Conçue pour une hémiplégie droite : **tout** se fait de la main gauche,
posée ou tenue, sans aucune torsion du poignet.

## Principes ergonomiques

1. **Le pouce fait le travail fin** : joystick + 4 boutons en arc de
   cercle autour de lui (rayon ≈ 3 tenons, distance d'un balayage de
   pouce sans déplacer la main).
2. **Index et majeur restent posés** sur leurs gâchettes (2 rangées de
   2) : aucun déplacement de doigt nécessaire.
3. **La paume presse** un gros bouton dans la poignée (mappé clic stick
   droit par défaut) : aucun doigt requis.
4. Les petits boutons (+/−/Home/Capture/TURBO/SHIFT) sont **hors de la
   zone de repos**, appuyés en déplaçant volontairement le pouce : pas
   d'appui accidentel.
5. La manette peut se **poser sur une table ou la cuisse** (fond plat,
   patins antidérapants) ou se sangler à la main (passant de sangle dans
   la poignée).

## Structure (≈ 1 h de montage)

### 1. Châssis

- Rectangle de poutres 1×15 et 1×11 (2 étages espacés de 3 tenons) :
  ~16 × 11 tenons, l'électronique loge entre les deux étages.
- Plaques Technic 5×11 vissées de goupilles sur l'étage bas : supports
  du DevKit ESP32, du TP4056 (port USB-C affleurant au bord arrière) et
  du support 18650.

### 2. Poignée (côté droit du châssis, saisie main gauche)

- 4 poutres coudées 3×5 assemblées en « D » vertical, entretoisées à
  4 tenons : diamètre de préhension ~35 mm.
- Le bouton de paume (bouton arcade 24 mm) traverse une poutre au fond
  du « D » : fermer la main l'actionne.
- Passant de sangle : axe de 5 avec arrêtoirs en bas de poignée.

### 3. Plateau du pouce (dessus, côté gauche)

- Plaque Technic 5×7 inclinée à ~20° vers l'utilisateur (2 connecteurs
  d'angle #2).
- Joystick au centre, collé sur la plaque ; les 4 boutons du pouce à
  10 h, 1 h, 4 h et 7 h autour de lui.
- TURBO et SHIFT sur le bord extérieur du plateau (atteints en étendant
  le pouce) ; +, −, Home, Capture sur une barrette près du bord haut.

### 4. Gâchettes (face avant, sous l'index et le majeur)

- 4 boutons tactiles 12 mm en 2 colonnes × 2 rangées, espacés de
  2 tenons, montés sur une plaque verticale.
- Option confort : recouvrir chaque bouton d'une **bascule LEGO**
  (poutre 1×3 sur pivot + élastique de rappel) pour élargir la surface
  d'appui et réduire la force nécessaire.

### 5. Éclairage et vibreur

- Bandeau WS2812B le long du bord supérieur, diffusé par des briques
  transparentes 1×2.
- Pastille vibrante collée **contre la poignée** (c'est la paume qui
  doit la sentir), fils vers le DRV2605L.

## Ajustements individuels

C'est l'intérêt du LEGO : **tout se déplace**.

- Trop de force nécessaire ? Allonger les bras de levier des bascules.
- Pouce court ? Rapprocher les 4 boutons (rayon 2 tenons).
- Spasticité ? Ajouter des rebords (poutres 1×2) autour des petits
  boutons pour éviter les appuis involontaires, et activer plutôt le
  remapping : mettre les fonctions importantes sur les gâchettes.
- Usage posé uniquement ? Supprimer la poignée et mettre le bouton de
  paume sous l'auriculaire.

Prendre des photos du montage final : elles servent de plan de
remontage.
