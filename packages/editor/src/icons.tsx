const paths = {
  logo: "M5 20h14M12 3v14M12 4l7 12h-7M12 7l-5 10h5",
  check: "M5 12l5 5 9-10",
  plus: "M12 5v14M5 12h14",
  chevron: "M6 9l6 6 6-6",
  external: "M7 17L17 7M8 7h9v9",
  gear: "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM19 12a7 7 0 0 0-.1-1.2l2-1.5-2-3.4-2.3 1a7 7 0 0 0-2-1.2L14.2 3h-4l-.4 2.7a7 7 0 0 0-2 1.2l-2.3-1-2 3.4 2 1.5a7 7 0 0 0 0 2.4l-2 1.5 2 3.4 2.3-1a7 7 0 0 0 2 1.2l.4 2.7h4l.4-2.7a7 7 0 0 0 2-1.2l2.3 1 2-3.4-2-1.5c.1-.4.1-.8.1-1.2z",
  heading: "M6 4v16M18 4v16M6 12h12",
  list: "M9 6h11M9 12h11M9 18h11M4 6h.01M4 12h.01M4 18h.01",
  numbered: "M10 6h10M10 12h10M10 18h10M4 5l1.5-1v4M4 14h3l-3 4h3",
  quote: "M5 7h5v5H7c0 2 1 3 3 3M14 7h5v5h-3c0 2 1 3 3 3",
  code: "M8 7l-5 5 5 5M16 7l5 5-5 5",
  info: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM12 11v6M12 7.5v.5",
  tabs: "M3 8a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z",
  sparkle: "M12 3l2 6 6 2-6 2-2 6-2-6-6-2 6-2z",
  image: "M4 5h16v14H4zM4 16l5-5 4 4 3-3 4 4M9 9h.01",
  table: "M4 5h16v14H4zM4 11h16M4 15h16M10 5v14",
  divider: "M4 12h16",
} as const;

export type IconName = keyof typeof paths;

export function Icon({ name, size = 16 }: { name: IconName; size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d={paths[name]} />
    </svg>
  );
}
