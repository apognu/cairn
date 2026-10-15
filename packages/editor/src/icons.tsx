const paths = {
  logo: "M3 19.5C3 17.6 7 16.5 11.5 16.5S20 17.6 20 19.5c0 1-3.5 1.5-8.5 1.5S3 20.5 3 19.5zM7 14.5c0-1.4 2.7-2.5 6-2.5s6 1 6 2.5c0 1.2-2.7 2-6 2s-6-.8-6-2zM5.5 10.2C5.5 9 7.5 8 10 8s4.5 1 4.5 2.2c0 1.1-2 1.8-4.5 1.8s-4.5-.7-4.5-1.8zM9.2 6.2c0-1.3 1.1-2.2 2.5-2.2s2.5.9 2.5 2.2c0 .9-1.1 1.6-2.5 1.6S9.2 7.1 9.2 6.2z",
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
  link: "M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1",
  grid: "M4 4h7v7H4zM13 4h7v7h-7zM4 13h7v7H4zM13 13h7v7h-7z",
  move: "M5 9l-3 3 3 3M19 9l3 3-3 3M2 12h20",
  eye: "M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7S2 12 2 12zM12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z",
  history: "M3 12a9 9 0 1 0 2.6-6.4M3 4v4.5h4.5M12 7.5V12l3 2",
  sliders: "M4 7h9M17 7h3M4 17h3M11 17h9M15 5v4M9 15v4",
  more: "M5 11a1 1 0 1 0 0 2 1 1 0 1 0 0-2zM12 11a1 1 0 1 0 0 2 1 1 0 1 0 0-2zM19 11a1 1 0 1 0 0 2 1 1 0 1 0 0-2z",
  lock: "M7 11h10a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2v-6a2 2 0 0 1 2-2zM8 11V8a4 4 0 0 1 8 0v3",
} as const;

export type IconName = keyof typeof paths;

export function Icon({ name, size = 16, className }: { name: IconName; size?: number; className?: string }) {
  return (
    <svg
      className={className}
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
