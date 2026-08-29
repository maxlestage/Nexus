# Site Nexus One

Site mobile-first du projet : présentation, schémas de câblage, liste du
matériel cochable, guide de montage et procédures d'appairage. Installable
comme application et consultable hors ligne — pensé pour être lu sur un
téléphone, à côté du fer à souder.

## Pile technique

Bun **uniquement** : gestionnaire de paquets, compilateur et serveur. Aucun
Node, aucun npm, aucun bundler tiers.

| | |
|---|---|
| Runtime, build, serveur | Bun 1.4.0 |
| Interface | React 19.2.8 |
| Typage | TypeScript 7.0.2 |
| Hébergement | Heroku (voir `DEPLOY.md`) |

## Commandes

```bash
bun install
bun run dev        # compile puis sert avec rechargement à chaud
bun run build      # compile dans dist/
bun run start      # sert dist/ (utilise $PORT si défini)
bun run typecheck  # tsc --noEmit
```

## Ce que fait la compilation

`build.ts` regroupe `src/main.tsx` et la feuille de style, empreinte les noms
de fichiers, réécrit `index.html`, recopie `public/`, puis génère `sw.js` avec
la liste exacte des ressources préchargées et une empreinte de version — de
sorte qu'un nouveau déploiement invalide automatiquement l'ancien cache.

## Mobile

- Mise en page à une colonne, cibles tactiles d'au moins 44 px
- `viewport-fit=cover` et marges de sécurité (encoche, barre d'accueil)
- Thème clair et sombre suivant le réglage du téléphone
- Manifeste PWA, icônes dont une masquable, raccourcis vers les sections
- Service worker : réseau d'abord pour le HTML, cache pour le reste
- Listes cochables conservées dans le stockage local, sans compte ni serveur
- Aucun défilement horizontal : les schémas larges défilent dans leur cadre

## Auteur

Conçu et développé par **Maxime Nathan Lestage**. Licence MIT.
