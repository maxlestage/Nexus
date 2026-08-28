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

/** Marque de la manette : un stick et l'arc de quatre boutons du pouce. */
export function BrandMark({ size = 26 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-hidden="true" focusable="false">
      <rect x="1.5" y="1.5" width="29" height="29" rx="8.5" fill="currentColor" opacity=".1" />
      <rect x="1.5" y="1.5" width="29" height="29" rx="8.5" fill="none" stroke="currentColor" strokeWidth="1.6" opacity=".45" />
      <circle cx="12.5" cy="16" r="4.6" fill="none" stroke="currentColor" strokeWidth="2" />
      <circle cx="12.5" cy="16" r="1.5" fill="currentColor" />
      <circle cx="22" cy="10.5" r="1.9" fill="currentColor" />
      <circle cx="25.5" cy="16" r="1.9" fill="currentColor" />
      <circle cx="22" cy="21.5" r="1.9" fill="currentColor" />
    </svg>
  );
}
