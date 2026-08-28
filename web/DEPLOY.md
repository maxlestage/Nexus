# Déploiement sur Heroku

Le site est servi par Bun seul : pas de Node, pas de npm, pas de serveur web
tiers. Heroku impose deux contraintes, toutes deux déjà prises en compte :
l'application doit écouter sur `$PORT` (`server.ts`) et le buildpack doit
savoir installer Bun.

## Le point qui coince

Le buildpack Node **officiel** d'Heroku ne connaît que npm, Yarn et pnpm — il
ne sait pas installer Bun. Il faut donc déclarer explicitement le buildpack
Bun communautaire :

```
https://github.com/jakeg/heroku-buildpack-bun
```

Il lit `.bun-version` (ici `1.4.0`), lance `bun install` puis `bun run build`,
et démarre le `Procfile`.

## Première mise en ligne

Le site vit dans le sous-dossier `web/` d'un dépôt qui contient aussi du Rust.
Heroku attend un `package.json` à la racine de ce qu'on lui pousse : on lui
pousse donc uniquement ce sous-dossier, avec `git subtree`.

```bash
# depuis la racine du dépôt
heroku create nexus-one            # choisissez un nom libre
heroku buildpacks:set https://github.com/jakeg/heroku-buildpack-bun -a nexus-one
heroku stack:set heroku-24 -a nexus-one

git subtree push --prefix web heroku master
heroku open -a nexus-one
```

Si Heroku refuse le push parce que la branche distante a divergé :

```bash
git push heroku "$(git subtree split --prefix web master)":refs/heads/master --force
```

## Mises à jour suivantes

```bash
git subtree push --prefix web heroku master
```

## Vérifier que tout va bien

```bash
heroku logs --tail -a nexus-one
curl https://nexus-one.herokuapp.com/healthz     # doit répondre « ok »
```

La route `/healthz` sert aussi de sonde de démarrage (déclarée dans `app.json`).

## Variante : garder la racine du dépôt

Si vous préférez pousser le dépôt entier plutôt qu'un sous-dossier, ajoutez le
buildpack monorepo **avant** celui de Bun et pointez-le sur `web/` :

```bash
heroku buildpacks:clear -a nexus-one
heroku buildpacks:add https://github.com/lstoll/heroku-buildpack-monorepo -a nexus-one
heroku buildpacks:add https://github.com/jakeg/heroku-buildpack-bun -a nexus-one
heroku config:set APP_BASE=web -a nexus-one
git push heroku master
```

## Ce qui se passe au déploiement

1. Le buildpack installe Bun 1.4.0 (lu dans `.bun-version`).
2. `bun install` récupère React et les types.
3. `bun run build` compile `src/` vers `dist/` : JavaScript et CSS empreintés,
   `index.html` réécrit, service worker généré avec la liste exacte des
   ressources à précharger.
4. Le `Procfile` lance `bun run server.ts`, qui écoute sur `$PORT`.

## Domaine personnalisé et HTTPS

```bash
heroku domains:add manette.exemple.fr -a nexus-one
heroku certs:auto:enable -a nexus-one
```

Le certificat automatique d'Heroku suffit ; le service worker et l'installation
de l'application sur mobile exigent HTTPS, qui est fourni d'office sur
`*.herokuapp.com`.
