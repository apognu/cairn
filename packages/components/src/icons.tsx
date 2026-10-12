// A small set of 24x24 stroke icons, enough for feature cards.
const paths = {
  rocket:
    "M5 15c-1 1-1.5 4-1.5 4s3-.5 4-1.5M12 15l-3-3a22 22 0 0 1 2-4 12 12 0 0 1 9-5c0 3-1 7-5 11a22 22 0 0 1-4 2zM9 12H5l2-3h4M12 15v4l3-2v-4",
  shield: "M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6z",
  bolt: "M13 3L5 14h6l-1 7 8-11h-6z",
  git: "M6 3v12M6 15a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM18 9a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM18 9c0 6-12 3-12 6",
  bell: "M6 17V11a6 6 0 0 1 12 0v6l2 2H4zM10 21a2 2 0 0 0 4 0",
  book: "M4 5a2 2 0 0 1 2-2h13v17H6a2 2 0 0 0-2 2zM4 19a2 2 0 0 1 2-2h13",
  cog: "M12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM19 12l2-1-2-4-2 1-2-2 1-2-4-1-1 2h-2L8 3 4 4l1 2-2 2 1 4 2-1 1 2-1 2 3 3 2-1 1 2 4-1-1-2 2-2 2 1z",
  cloud: "M7 18a4 4 0 0 1-.5-8A6 6 0 0 1 18 9a4.5 4.5 0 0 1 0 9z",
  lock: "M6 11h12v9H6zM8 11V8a4 4 0 0 1 8 0v3",
  chart: "M4 20V4M4 20h16M8 16v-4M12 16V8M16 16v-6",
  plug: "M9 3v5M15 3v5M6 8h12v3a6 6 0 0 1-12 0zM12 17v4",
  star: "M12 3l2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1L3.2 9.5l6.1-.9z",
} as const;

export const iconNames = Object.keys(paths) as [keyof typeof paths, ...(keyof typeof paths)[]];

export function Icon({ name }: { name: string }) {
  const path = paths[name as keyof typeof paths] ?? paths.star;

  return (
    <svg
      width="20"
      height="20"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d={path} />
    </svg>
  );
}
