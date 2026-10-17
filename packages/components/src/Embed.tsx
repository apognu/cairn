import { defineComponent, z } from "@cairn/sdk";
import { useRenderEnv } from "./env";

// Only the video ID is taken from the address; the player address is built here, so a crafted URL cannot point the frame elsewhere.
// Regular expressions, not `URL`: the publish renderer's V8 has no web APIs.
const sources: { pattern: RegExp; player: (match: RegExpMatchArray) => string }[] = [
  {
    pattern:
      /^https?:\/\/(?:(?:www|m)\.)?(?:youtube\.com\/(?:watch\?(?:[^#]*&)?v=|embed\/|shorts\/|live\/)|youtube-nocookie\.com\/embed\/|youtu\.be\/)([\w-]{11})(?![\w-])/,
    // The privacy-enhanced player sets no cookie until the reader presses play.
    player: ([, id]) => `https://www.youtube-nocookie.com/embed/${id}`,
  },
  {
    pattern: /^https?:\/\/(?:www\.)?vimeo\.com\/(\d+)(?:\/([\da-f]+))?(?![\w-])/,
    // An unlisted video carries its key after the ID.
    player: ([, id, key]) => `https://player.vimeo.com/video/${id}${key ? `?h=${key}` : ""}`,
  },
  {
    pattern: /^https?:\/\/player\.vimeo\.com\/video\/(\d+)(?:\?(?:[^#]*&)?h=([\da-f]+))?(?![\w-])/,
    player: ([, id, key]) => `https://player.vimeo.com/video/${id}${key ? `?h=${key}` : ""}`,
  },
  {
    pattern: /^https?:\/\/(?:www\.)?loom\.com\/(?:share|embed)\/([\da-f]{32})(?![\w-])/,
    player: ([, id]) => `https://www.loom.com/embed/${id}`,
  },
];

/** The player address for a YouTube, Vimeo or Loom video's address; undefined for any other address. */
export function embedPlayer(url: string): string | undefined {
  for (const { pattern, player } of sources) {
    const match = url.trim().match(pattern);
    if (match) return player(match);
  }
  return undefined;
}

function Embed({ url, title }: { url: string; title: string }) {
  const { editing } = useRenderEnv();
  const player = embedPlayer(url);

  if (!player) {
    return editing ? (
      <div className="embed" data-empty>
        Paste a YouTube, Vimeo or Loom address in the settings panel
      </div>
    ) : null;
  }

  return (
    <div className="embed">
      <iframe
        src={player}
        title={title}
        loading="lazy"
        // Players need their own scripts and storage; popups let "watch on YouTube" open. Nothing else: no top navigation, no forms.
        sandbox="allow-scripts allow-same-origin allow-presentation allow-popups"
        allow="autoplay; encrypted-media; fullscreen; picture-in-picture"
        allowFullScreen
        // YouTube refuses to play in a frame that sends no referrer.
        referrerPolicy="strict-origin-when-cross-origin"
      />
    </div>
  );
}

/** A YouTube, Vimeo or Loom video, played in the page. Other hosts are refused, so a page only ever frames these three. */
export const embed = defineComponent({
  name: "embed",
  version: 1,
  title: "Video",
  description: "A YouTube, Vimeo or Loom video, played in the page.",
  props: z.object({
    url: z
      .string()
      .refine(
        (url) => !url.trim() || embedPlayer(url) !== undefined,
        "only YouTube, Vimeo and Loom videos can be embedded",
      )
      .default("")
      .meta({ title: "Video address", description: "The address of the video's page, or its share link." }),
    title: z
      .string()
      .default("Video")
      .meta({ title: "Title", description: "What the video is about, read by screen readers." }),
  }),
  render: ({ url, title }) => <Embed url={url} title={title} />,
});
