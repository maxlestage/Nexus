# Déploiement sur Heroku

## L'erreur à ne pas refaire

```
-----> Using buildpack: heroku/nodejs
 !     ERROR: Application not supported by 'heroku/nodejs' buildpack
```

Ce message veut dire que le buildpack Bun **n'a pas été appliqué**. Le
buildpack Node officiel d'Heroku ne sait installer que npm, Yarn et pnpm : il
ignore Bun et échoue. Il faut le remplacer, en une commande, **avant** de
pousser :

```bash
heroku buildpacks:set https://github.com/jakeg/heroku-buildpack-bun -a VOTRE-APP
```

C'est la seule étape qu'Heroku ne peut pas deviner : aucun fichier du dépôt ne
peut la remplacer.

## Mise en ligne complète

Le dépôt entier est déployable tel quel : sa racine contient `package.json`,
`Procfile` et `.bun-version`, que le buildpack Bun cherche à cet endroit.

```bash
heroku create nexus-one                     # choisissez un nom libre
heroku buildpacks:set https://github.com/jakeg/heroku-buildpack-bun -a nexus-one
heroku stack:set heroku-24 -a nexus-one

git push heroku master
heroku open -a nexus-one
```

Si votre branche locale s'appelle autrement que celle attendue par Heroku :

```bash
git push heroku master:main      # adaptez selon le cas
```

## Mises à jour suivantes

```bash
git push heroku master
```

## Ce qui se passe pendant le build

1. Le buildpack lit `.bun-version` à la racine et installe Bun 1.4.0.
2. `bun install` à la racine : l'espace de travail Bun (`workspaces: ["web"]`)
   récupère React et les types pour `web/`.
3. `bun run build` à la racine délègue à `web/` et produit `web/dist/` :
   JavaScript et CSS empreintés, `index.html` réécrit, service worker généré
   avec la liste exacte des ressources préchargées.
4. Le `Procfile` lance `bun run web/server.ts`, qui écoute sur `$PORT`.

Le serveur résout `dist/` à partir de l'emplacement de `server.ts`, pas du
dossier courant : il démarre donc correctement quel que soit le répertoire de
lancement.

## Vérifier

```bash
heroku logs --tail -a nexus-one
curl https://nexus-one-XXXX.herokuapp.com/healthz    # doit répondre « ok »
```

`/healthz` sert aussi de sonde de démarrage, déclarée dans `app.json`.

## Variante : ne pousser que le sous-dossier

Si vous préférez qu'Heroku ne reçoive pas le code Rust, poussez uniquement
`web/`. Les fichiers de déploiement présents à la racine deviennent alors
inutiles, ceux de `web/` prennent le relais.

```bash
git subtree push --prefix web heroku master
```

## Domaine personnalisé et HTTPS

```bash
heroku domains:add manette.exemple.fr -a nexus-one
heroku certs:auto:enable -a nexus-one
```

HTTPS est indispensable : sans lui, le service worker ne s'installe pas et le
site n'est pas installable comme application sur téléphone. Les domaines
`*.herokuapp.com` sont servis en HTTPS d'office.
