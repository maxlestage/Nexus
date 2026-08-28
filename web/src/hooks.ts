import { useCallback, useEffect, useState } from "react";

/** Persiste un état dans localStorage, en tolérant les contextes où il est inaccessible. */
export function usePersistentSet(key: string): {
  has: (id: string) => boolean;
  toggle: (id: string) => void;
  clear: () => void;
  size: number;
} {
  const [ids, setIds] = useState<Set<string>>(() => {
    try {
      const raw = localStorage.getItem(key);
      if (!raw) return new Set();
      const parsed: unknown = JSON.parse(raw);
      return Array.isArray(parsed) ? new Set(parsed.filter((v) => typeof v === "string")) : new Set();
    } catch {
      return new Set();
    }
  });

  useEffect(() => {
    try {
      localStorage.setItem(key, JSON.stringify([...ids]));
    } catch {
      /* mode privé, stockage bloqué : l'état reste en mémoire */
    }
  }, [key, ids]);

  const toggle = useCallback((id: string) => {
    setIds((prev) => {
      const next = new Set(prev);
      if (!next.delete(id)) next.add(id);
      return next;
    });
  }, []);

  const clear = useCallback(() => setIds(new Set()), []);
  const has = useCallback((id: string) => ids.has(id), [ids]);

  return { has, toggle, clear, size: ids.size };
}

/** Renvoie l'id de la section actuellement à l'écran. */
export function useActiveSection(ids: readonly string[]): string {
  const [active, setActive] = useState<string>("");

  useEffect(() => {
    const seen = new Map<string, number>();
    const observer = new IntersectionObserver(
      (entries) => {
        for (const e of entries) seen.set(e.target.id, e.intersectionRatio);
        let best = "";
        let bestRatio = 0;
        for (const id of ids) {
          const ratio = seen.get(id) ?? 0;
          if (ratio > bestRatio) {
            bestRatio = ratio;
            best = id;
          }
        }
        if (best) setActive(best);
      },
      { rootMargin: "-72px 0px -55% 0px", threshold: [0, 0.15, 0.4, 0.75, 1] },
    );
    for (const id of ids) {
      const el = document.getElementById(id);
      if (el) observer.observe(el);
    }
    return () => observer.disconnect();
  }, [ids]);

  return active;
}

type InstallEvent = Event & { prompt: () => Promise<void> };

/** Expose l'invite d'installation PWA quand le navigateur la propose. */
export function useInstallPrompt(): { canInstall: boolean; install: () => void; dismiss: () => void } {
  const [evt, setEvt] = useState<InstallEvent | null>(null);
  const [dismissed, setDismissed] = useState(false);

  useEffect(() => {
    const onPrompt = (e: Event) => {
      e.preventDefault();
      setEvt(e as InstallEvent);
    };
    const onInstalled = () => setEvt(null);
    window.addEventListener("beforeinstallprompt", onPrompt);
    window.addEventListener("appinstalled", onInstalled);
    return () => {
      window.removeEventListener("beforeinstallprompt", onPrompt);
      window.removeEventListener("appinstalled", onInstalled);
    };
  }, []);

  const install = useCallback(() => {
    void evt?.prompt();
    setEvt(null);
  }, [evt]);

  return { canInstall: evt !== null && !dismissed, install, dismiss: () => setDismissed(true) };
}
