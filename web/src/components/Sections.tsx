import { useRef, useState } from "react";
import { ARCHITECTURE, BOM, FEATURES, PAIRING, REPO, STEPS, ZONES } from "../content.ts";
import { usePersistentSet } from "../hooks.ts";
import { Icon } from "./Icons.tsx";

export function Hero() {
  return (
    <section id="intro" className="hero">
      <p className="eyebrow">Manette accessible · matériel libre</p>
      <h1>Jouer d'une seule main.</h1>
      <p className="lede">
        Une manette compatible Nintendo Switch et PC, construite en LEGO Technic autour d'un
        ESP32. Conçue pour une hémiplégie droite : joystick, gâchettes et boutons sont tous
        atteignables de la main gauche, sans bouger le poignet.
      </p>
      <div className="badges">
        <span className="badge">Switch</span>
        <span className="badge">PC · macOS</span>
        <span className="badge">App iPhone</span>
        <span className="badge">~60–90 €</span>
        <span className="badge">100 % Rust</span>
      </div>
      <div className="cta-row">
        <a className="btn btn-primary" href="#materiel">Voir la liste du matériel</a>
        <a className="btn btn-secondary" href={REPO} target="_blank" rel="noreferrer">
          Le code <Icon name="external" size={17} />
        </a>
      </div>
    </section>
  );
}

export function Features() {
  return (
    <section id="fonctions">
      <hr className="rule" />
      <p className="eyebrow" style={{ marginTop: 22 }}>Ce qu'elle fait</p>
      <h2>Une manette complète, pas une adaptation au rabais</h2>
      <p className="sub">
        Tout ce qu'offre une manette du commerce, réparti autrement — plus ce qu'aucune
        ne propose : le remappage depuis le téléphone, et une coque qu'on ajuste à la main
        qui la tient.
      </p>
      <div className="grid grid-3">
        {FEATURES.map((f) => (
          <article className="card" key={f.title}>
            <span className="card-icon"><Icon name={f.icon} /></span>
            <h3>{f.title}</h3>
            <p>{f.body}</p>
          </article>
        ))}
      </div>
    </section>
  );
}

export function Hand() {
  return (
    <section id="prise-en-main">
      <hr className="rule" />
      <p className="eyebrow" style={{ marginTop: 22 }}>Prise en main</p>
      <h2>Ce que fait chaque doigt</h2>
      <p className="sub">
        La main reste posée : aucun doigt n'a besoin de se déplacer pour atteindre une commande
        courante. Les fonctions rares sont volontairement mises hors de portée immédiate.
      </p>
      <div className="zones">
        {ZONES.map((z, i) => (
          <div className="zone" key={i}>
            <span className="finger">{z.finger}</span>
            <span className="what">{z.what}</span>
            <span className="maps">{z.maps}</span>
            {z.note ? <span className="note">{z.note}</span> : null}
          </div>
        ))}
      </div>
    </section>
  );
}

function Diagram({ src, alt, label }: { src: string; alt: string; label: string }) {
  const dialog = useRef<HTMLDialogElement>(null);
  return (
    <>
      <div className="sheet">
        <img src={src} alt={alt} loading="lazy" />
      </div>
      <p className="sheet-hint">
        <Icon name="move" size={16} /> Faites glisser le schéma horizontalement
      </p>
      <button className="zoom-btn" onClick={() => dialog.current?.showModal()}>
        Ouvrir en plein écran
      </button>
      <dialog className="viewer" ref={dialog} aria-label={label}>
        <div className="viewer-inner">
          <div className="viewer-bar">
            <span>{label}</span>
            <button className="viewer-close" onClick={() => dialog.current?.close()}>Fermer</button>
          </div>
          <div className="viewer-scroll">
            <img src={src} alt={alt} />
          </div>
        </div>
      </dialog>
    </>
  );
}

export function Wiring() {
  return (
    <section id="cablage">
      <hr className="rule" />
      <p className="eyebrow" style={{ marginTop: 22 }}>Câblage</p>
      <h2>Le schéma, consultable au poste de soudage</h2>
      <p className="sub">
        Les couleurs des traits correspondent aux fonctions des fils : rouge 5 V, orange 3,3 V,
        bleu signal, vert I2C, violet données LED, cyan analogique.
      </p>
      <Diagram
        src="/wiring-diagram.svg"
        alt="Schéma de câblage complet : boutons, périphériques et chaîne d'alimentation autour de l'ESP32"
        label="Schéma d'ensemble"
      />
      <h3 style={{ marginTop: 30 }}>Les trois montages délicats</h3>
      <p className="sub">Le reste du câblage est répétitif.</p>
      <Diagram
        src="/wiring-details.svg"
        alt="Détails : pont diviseur batterie, résistance de rappel sur GPIO39, câblage type d'un bouton"
        label="Détails de câblage"
      />
    </section>
  );
}

const EUR = new Intl.NumberFormat("fr-FR", { style: "currency", currency: "EUR", maximumFractionDigits: 0 });

export function Bom() {
  const bought = usePersistentSet("nexus-bom");
  const min = BOM.reduce((s, i) => s + i.min, 0);
  const max = BOM.reduce((s, i) => s + i.max, 0);

  return (
    <section id="materiel">
      <hr className="rule" />
      <p className="eyebrow" style={{ marginTop: 22 }}>Matériel</p>
      <h2>La liste de courses</h2>
      <p className="sub">
        Cochez au fur et à mesure : votre liste est conservée sur cet appareil, même hors ligne.
      </p>

      <div className="progress">
        <div className="progress-track">
          <div className="progress-fill" style={{ width: `${(bought.size / BOM.length) * 100}%` }} />
        </div>
        <span className="progress-label">{bought.size}/{BOM.length}</span>
        {bought.size > 0 ? (
          <button className="reset-btn" onClick={bought.clear}>Effacer</button>
        ) : null}
      </div>

      <div className="checklist">
        {BOM.map((item) => {
          const done = bought.has(item.id);
          return (
            <button
              className="check"
              key={item.id}
              data-done={done}
              aria-pressed={done}
              onClick={() => bought.toggle(item.id)}
            >
              <span className="box"><Icon name="check" size={15} /></span>
              <span className="check-body">
                <span className="check-title">
                  {item.qty > 1 ? `${item.qty} × ` : ""}{item.name}
                </span>
                <span className="check-meta">{item.role}</span>
                {item.critical ? <span className="check-warn">{item.critical}</span> : null}
              </span>
              <span className="check-price">
                {item.min === item.max ? EUR.format(item.min) : `${item.min}–${item.max} €`}
              </span>
            </button>
          );
        })}
      </div>

      <div className="total">
        <span>Total électronique, hors LEGO et outillage</span>
        <b>{min}–{max} €</b>
      </div>
    </section>
  );
}

export function Build() {
  const done = usePersistentSet("nexus-steps");
  return (
    <section id="montage">
      <hr className="rule" />
      <p className="eyebrow" style={{ marginTop: 22 }}>Montage</p>
      <h2>Dans cet ordre, en testant à chaque étape</h2>
      <p className="sub">
        Chaque étape se vérifie avant de passer à la suivante : c'est ce qui évite de rouvrir
        une coque entière pour un fil dessoudé.
      </p>
      <div className="progress">
        <div className="progress-track">
          <div className="progress-fill" style={{ width: `${(done.size / STEPS.length) * 100}%` }} />
        </div>
        <span className="progress-label">{done.size}/{STEPS.length}</span>
        {done.size > 0 ? <button className="reset-btn" onClick={done.clear}>Effacer</button> : null}
      </div>
      <div className="checklist">
        {STEPS.map((s, i) => {
          const isDone = done.has(s.id);
          return (
            <button
              className="check"
              key={s.id}
              data-done={isDone}
              aria-pressed={isDone}
              onClick={() => done.toggle(s.id)}
            >
              <span className="step-num">{isDone ? <Icon name="check" size={14} /> : i + 1}</span>
              <span className="check-body">
                <span className="check-title">{s.title}</span>
                <span className="check-meta">{s.body}</span>
              </span>
            </button>
          );
        })}
      </div>
    </section>
  );
}

export function Pairing() {
  const [tab, setTab] = useState(PAIRING[0]!.id);
  const current = PAIRING.find((p) => p.id === tab) ?? PAIRING[0]!;
  return (
    <section id="appairage">
      <hr className="rule" />
      <p className="eyebrow" style={{ marginTop: 22 }}>Appairage</p>
      <h2>La connecter</h2>
      <div className="tabs" role="tablist">
        {PAIRING.map((p) => (
          <button
            key={p.id}
            role="tab"
            aria-selected={p.id === tab}
            onClick={() => setTab(p.id)}
          >
            {p.label}
          </button>
        ))}
      </div>
      <ol className="plain">
        {current.steps.map((s, i) => <li key={i}>{s}</li>)}
      </ol>
      {current.note ? <p className="note">{current.note}</p> : null}
    </section>
  );
}

export function Code() {
  return (
    <section id="code">
      <hr className="rule" />
      <p className="eyebrow" style={{ marginTop: 22 }}>Le code</p>
      <h2>Quatre modules, un seul protocole</h2>
      <p className="sub">
        Le firmware et l'application partagent le même code de protocole : impossible que
        les deux divergent.
      </p>
      <div className="grid">
        {ARCHITECTURE.map((m) => (
          <article className="mod" key={m.name}>
            <div className="mod-head">
              <span className="mod-name">{m.name}</span>
              <span className="mod-lang">{m.lang}</span>
            </div>
            <p>{m.body}</p>
            <span className="pill" data-ok={m.tests.includes("tests")}>{m.tests}</span>
          </article>
        ))}
      </div>
      <div className="cta-row" style={{ marginTop: 18 }}>
        <a className="btn btn-secondary" href={REPO} target="_blank" rel="noreferrer">
          Ouvrir le dépôt <Icon name="external" size={17} />
        </a>
      </div>
    </section>
  );
}
