import { expect, test } from "bun:test";
import { embedPlayer } from "./Embed";

test("frames only YouTube, Vimeo and Loom, through their players", () => {
  const loom = "0123456789abcdef0123456789abcdef";
  const cases: [string, string | undefined][] = [
    ["https://www.youtube.com/watch?v=dQw4w9WgXcQ", "https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ"],
    [
      "https://youtube.com/watch?feature=share&v=dQw4w9WgXcQ&t=42",
      "https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ",
    ],
    ["https://m.youtube.com/watch?v=dQw4w9WgXcQ", "https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ"],
    ["  https://youtu.be/dQw4w9WgXcQ?si=abc  ", "https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ"],
    ["https://www.youtube.com/shorts/dQw4w9WgXcQ", "https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ"],
    ["https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ", "https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ"],
    ["https://vimeo.com/76979871", "https://player.vimeo.com/video/76979871"],
    ["https://vimeo.com/76979871/8f2a1c9b3d", "https://player.vimeo.com/video/76979871?h=8f2a1c9b3d"],
    ["https://player.vimeo.com/video/76979871?h=8f2a1c9b3d", "https://player.vimeo.com/video/76979871?h=8f2a1c9b3d"],
    [`https://www.loom.com/share/${loom}`, `https://www.loom.com/embed/${loom}`],
    [`https://loom.com/embed/${loom}?sid=1`, `https://www.loom.com/embed/${loom}`],
    // Other hosts, look-alikes and addresses with no video in them.
    ["https://example.com/watch?v=dQw4w9WgXcQ", undefined],
    ["https://youtube.com.evil.test/watch?v=dQw4w9WgXcQ", undefined],
    ["https://evil.test/youtu.be/dQw4w9WgXcQ", undefined],
    ["https://www.youtube.com/watch?v=short", undefined],
    ["https://www.youtube.com/watch?v=dQw4w9WgXcQextra", undefined],
    ["https://vimeo.com/channels/staffpicks", undefined],
    ["javascript:alert(1)//youtu.be/dQw4w9WgXcQ", undefined],
    ["", undefined],
  ];

  for (const [url, expected] of cases) expect([url, embedPlayer(url)]).toEqual([url, expected]);
});
