import { expect, test } from "bun:test";
import { looksLikeMarkdown, markdownToHtml } from "./markdown";

test("tells Markdown from plain words", () => {
  const cases: [string, boolean][] = [
    ["Just a sentence, with a dash - and a star * in it.", false],
    ["Version 2.0 is out.", false],
    ["## Install", true],
    ["- one\n- two", true],
    ["1. first\n2. second", true],
    ["> quoted", true],
    ["```sh\nnpm i\n```", true],
    ["| a | b |\n| --- | --- |\n| 1 | 2 |", true],
    ["See [the guide](https://example.com).", true],
    ["Run `npm install` first.", true],
    ["This is **important**.", true],
  ];

  for (const [text, expected] of cases) expect([text, looksLikeMarkdown(text)]).toEqual([text, expected]);
});

test("converts Markdown to the HTML the editor reads", () => {
  const cases: [string, string][] = [
    // With a `#` heading, every heading moves down a level, none below 4.
    ["# Title\n\n## Part\n\n#### Deep", "<h2>Title</h2>\n<h3>Part</h3>\n<h4>Deep</h4>"],
    ["## Part\n\n### Sub\n\n##### Deeper", "<h2>Part</h2>\n<h3>Sub</h3>\n<h4>Deeper</h4>"],
    ["```js\nlet a = 1;\n```", '<pre><code class="language-js">let a = 1;</code></pre>'],
    ["| a | b |\n| --- | --- |\n| 1 | 2 |", "<table>"],
    ["~~gone~~ and [link](https://example.com)", '<p><del>gone</del> and <a href="https://example.com">link</a></p>'],
    // Raw HTML is text, never markup.
    ["<script>alert(1)</script>", "&lt;script&gt;alert(1)&lt;/script&gt;"],
  ];

  for (const [markdown, html] of cases) expect(markdownToHtml(markdown)).toContain(html);
});
