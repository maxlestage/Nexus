type Props = { name: string; size?: number };

/** Pictogrammes au trait, hérités de la couleur courante. */
export function Icon({ name, size = 24 }: Props) {
  const paths: Record<string, React.ReactNode> = {
    hand: (
      <>
        <path d="M9 11V5.5a1.5 1.5 0 0 1 3 0V11" />
        <path d="M12 11V4.5a1.5 1.5 0 0 1 3 0V11" />
        <path d="M15 11V6.5a1.5 1.5 0 0 1 3 0V15a6 6 0 0 1-6 6h-1a6 6 0 0 1-5.2-3L4 15.2a1.6 1.6 0 0 1 2.7-1.7L9 16" />
      </>
    ),
    layers: (
      <>
        <path d="m12 3 9 5-9 5-9-5 9-5Z" />
        <path d="m3 13 9 5 9-5" />
      </>
    ),
    switch: (
      <>
        <rect x="2.5" y="6" width="19" height="12" rx="4" />
        <path d="M8 10v4M6 12h4" />
        <circle cx="16.5" cy="12" r="1.2" />
      </>
    ),
    desktop: (
      <>
        <rect x="2.5" y="4" width="19" height="13" rx="2" />
        <path d="M9 21h6M12 17v4" />
      </>
    ),
    phone: (
      <>
        <rect x="6" y="2.5" width="12" height="19" rx="3" />
        <path d="M11 18.5h2" />
      </>
    ),
    wave: (
      <>
        <path d="M3 12h2l2-5 3 10 3-13 3 16 2-8h3" />
      </>
    ),
    bolt: <path d="M13 2 4 14h7l-1 8 9-12h-7l1-8Z" />,
    battery: (
      <>
        <rect x="2.5" y="7" width="16" height="10" rx="2.5" />
        <path d="M21.5 10.5v3" />
        <path d="M6 10.5v3M9.5 10.5v3" />
      </>
    ),
    check: <path d="m4.5 12.5 5 5 10-11" />,
    external: (
      <>
        <path d="M14 3h7v7" />
        <path d="M21 3 10 14" />
        <path d="M19 14v6a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1h6" />
      </>
    ),
    move: (
      <>
        <path d="M5 9 2 12l3 3M19 9l3 3-3 3" />
        <path d="M2 12h20" />
      </>
    ),
  };

  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.7"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      {paths[name] ?? null}
    </svg>
  );
}

/** Marque de la manette : le stick et les quatre boutons du pouce.
 *  Reprend le dessin de l'icône de l'application (web/public/favicon.svg). */
export function BrandMark({ size = 26 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 512 512" aria-hidden="true" focusable="false">
      <defs>
        <linearGradient id="bm-bg" x1="0" y1="0" x2="0.55" y2="1">
          <stop offset="0" stopColor="#334252" />
          <stop offset="1" stopColor="#111820" />
        </linearGradient>
        <linearGradient id="bm-cu" x1="0.1" y1="0" x2="0.9" y2="1">
          <stop offset="0" stopColor="#f6bc90" />
          <stop offset="0.5" stopColor="#e08a4e" />
          <stop offset="1" stopColor="#b45a22" />
        </linearGradient>
      </defs>
      <rect width="512" height="512" rx="118" fill="url(#bm-bg)" />
      <circle cx="256" cy="256" r="80" fill="none" stroke="url(#bm-cu)" strokeWidth="34" />
      <path
        d="M176 256 A80 80 0 0 1 228.8 180.8"
        fill="none"
        stroke="#fff"
        strokeOpacity="0.34"
        strokeWidth="10.2"
        strokeLinecap="round"
      />
      <circle cx="256" cy="256" r="23" fill="url(#bm-cu)" />
      <circle cx="362.1" cy="149.9" r="32" fill="#fff" />
      <circle cx="362.1" cy="362.1" r="32" fill="#fff" />
      <circle cx="149.9" cy="362.1" r="32" fill="#fff" />
      <circle cx="149.9" cy="149.9" r="32" fill="#fff" />
    </svg>
  );
}
