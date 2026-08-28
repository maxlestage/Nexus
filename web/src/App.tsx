import { useEffect } from "react";
import { useActiveSection, useInstallPrompt } from "./hooks.ts";
import { BrandMark } from "./components/Icons.tsx";
import { Bom, Build, Code, Features, Hand, Hero, Pairing, Wiring } from "./components/Sections.tsx";
import { REPO } from "./content.ts";

const NAV = [
  { id: "fonctions", label: "Fonctions" },
  { id: "prise-en-main", label: "Prise en main" },
  { id: "cablage", label: "Câblage" },
  { id: "materiel", label: "Matériel" },
  { id: "montage", label: "Montage" },
  { id: "appairage", label: "Appairage" },
  { id: "code", label: "Code" },
] as const;

const NAV_IDS = NAV.map((n) => n.id);

export function App() {
  const active = useActiveSection(NAV_IDS);
  const { canInstall, install, dismiss } = useInstallPrompt();

  // Le navigateur résout l'ancre avant que React ait monté les sections :
  // sans cela, les raccourcis du manifeste (/#materiel…) restent en haut.
  useEffect(() => {
    const id = decodeURIComponent(location.hash.slice(1));
    if (!id) return;
    requestAnimationFrame(() => document.getElementById(id)?.scrollIntoView());
  }, []);

  return (
    <>
      <header className="hdr">
        <div className="hdr-bar">
          <span className="brand">
            <span className="brand-mark"><BrandMark /></span>
            Nexus One
          </span>
          <span className="hdr-spacer" />
          <a className="ghost" href={REPO} target="_blank" rel="noreferrer">GitHub</a>
        </div>
        <nav className="sections" aria-label="Sections">
          {NAV.map((n) => (
            <a key={n.id} href={`#${n.id}`} aria-current={active === n.id}>
              {n.label}
            </a>
          ))}
        </nav>
      </header>

      <main>
        <Hero />
        <Features />
        <Hand />
        <Wiring />
        <Bom />
        <Build />
        <Pairing />
        <Code />

        <footer>
          <p>
            Projet personnel en matériel libre. Le protocole Pro Controller provient du
            travail de rétro-ingénierie de la communauté ; Nintendo et Switch sont des
            marques de Nintendo, sans lien avec ce projet.
          </p>
          <p className="offline-tag">
            <span className="dot" /> Ce site fonctionne hors ligne une fois consulté.
          </p>
        </footer>
      </main>

      {canInstall ? (
        <div className="install" role="dialog" aria-label="Installer l'application">
          <p>
            <strong>Installer le guide</strong>
            Disponible hors ligne, dans l'atelier.
          </p>
          <button className="btn btn-primary" onClick={install}>Installer</button>
          <button className="icon-close" onClick={dismiss} aria-label="Masquer">×</button>
        </div>
      ) : null}
    </>
  );
}
