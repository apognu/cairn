import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { useSetDescription } from "./queries";
import type { Page } from "./tree";

/**
 * The page's description, written in place where readers see it: under the title, as the lead. It is one sentence or two, so Enter
 * moves on to the page body instead of starting a new line.
 */
export function LeadField({ page }: { page: Page }) {
  const save = useSetDescription();
  const field = useRef<HTMLTextAreaElement>(null);
  const [text, setText] = useState(page.description ?? "");
  const saved = useRef(page.description ?? "");

  // A change made in Page settings shows here, unless the author is typing in this field.
  useEffect(() => {
    saved.current = page.description ?? "";
    if (document.activeElement !== field.current) setText(page.description ?? "");
  }, [page.description]);

  // As tall as its text: it reads as a paragraph, not a box.
  // biome-ignore lint/correctness/useExhaustiveDependencies: measured again whenever the text changes
  useLayoutEffect(() => {
    const element = field.current;
    if (!element) return;
    element.style.height = "auto";
    element.style.height = `${element.scrollHeight}px`;
  }, [text]);

  const commit = () => {
    const next = text.trim();
    if (next === saved.current) return;
    saved.current = next;
    save.mutate({ id: page.id, description: next });
  };

  return (
    <>
      <textarea
        ref={field}
        // `doc-lead` draws it as the published lead; only its placeholder and focus show it can be edited.
        className="doc-lead block w-full p-0 border-0 bg-transparent font-[inherit] resize-none overflow-hidden placeholder:text-muted placeholder:opacity-60 focus-visible:outline-none"
        aria-label="Description"
        rows={1}
        maxLength={300}
        // Short: the field is one line high until it holds text, and a long placeholder would be cut off.
        placeholder="Add a description…"
        title="The lead of the page, also shown on link cards and in search results"
        value={text}
        onChange={(e) => setText(e.target.value.replace(/\n/g, " "))}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key !== "Enter") return;
          e.preventDefault();
          commit();
          document.querySelector<HTMLElement>(".ProseMirror")?.focus();
        }}
      />
      {save.error && (
        <p role="alert" className="text-danger">
          {save.error.message}
        </p>
      )}
    </>
  );
}
