import { docTitleClass } from "@cairn/renderer";
import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import { useRenamePage } from "./queries";
import type { Page } from "./tree";

/**
 * The page's title, renamed in place where readers see it. A rename can change the page's URL, so it is never saved on the way out:
 * once the text changes, the choice to keep the old URL as a redirect shows under it, Enter renames and Escape puts the title back.
 * Leaving the field keeps the change pending, so the checkbox can be clicked.
 */
export function TitleField({ page }: { page: Page }) {
  const rename = useRenamePage();
  const field = useRef<HTMLTextAreaElement>(null);
  const [text, setText] = useState(page.title);
  const [keep, setKeep] = useState(true);
  const checkId = useId();
  const changed = text.trim() !== page.title;

  // A rename from the page tree shows here, unless the author has a change of their own pending.
  // biome-ignore lint/correctness/useExhaustiveDependencies: only a new title from outside resets the field
  useEffect(() => {
    if (!changed || document.activeElement !== field.current) setText(page.title);
  }, [page.title]);

  // As tall as its text: a long title wraps as the heading does.
  // biome-ignore lint/correctness/useExhaustiveDependencies: measured again whenever the text changes
  useLayoutEffect(() => {
    const element = field.current;
    if (!element) return;
    element.style.height = "auto";
    element.style.height = `${element.scrollHeight}px`;
  }, [text]);

  const cancel = () => {
    setText(page.title);
    rename.reset();
    field.current?.blur();
  };
  const submit = () => {
    const title = text.trim();
    if (!title || !changed) return cancel();
    if (rename.isPending) return;

    rename.mutate(
      { id: page.id, title, redirect: keep },
      {
        onSuccess: () => {
          setKeep(true);
          // On to the description, as Enter there moves on to the page body.
          document.querySelector<HTMLElement>('textarea[aria-label="Description"]')?.focus();
        },
      },
    );
  };

  return (
    <>
      <h1 className={docTitleClass}>
        <textarea
          ref={field}
          className="block w-full p-0 m-0 border-0 bg-transparent text-inherit [font:inherit] tracking-[inherit] resize-none overflow-hidden placeholder:text-muted placeholder:opacity-60 focus-visible:outline-none"
          aria-label="Page title"
          rows={1}
          placeholder="Page title"
          value={text}
          onChange={(e) => setText(e.target.value.replace(/\n/g, " "))}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              submit();
            } else if (e.key === "Escape") {
              e.preventDefault();
              cancel();
            }
          }}
        />
      </h1>
      {changed && (
        <div className="flex flex-wrap items-center gap-x-4 gap-y-1 -mt-2 mb-4 text-ui leading-[1.4]">
          <label htmlFor={checkId} className="flex items-center gap-2 text-text">
            <input
              id={checkId}
              type="checkbox"
              className="w-4 h-4 m-0"
              checked={keep}
              onChange={(e) => setKeep(e.target.checked)}
            />
            Keep old URL as redirect
          </label>
          <span className="text-muted text-help">Enter to rename, Escape to cancel</span>
        </div>
      )}
      {rename.error && (
        <p role="alert" className="-mt-2 mb-4 text-danger text-ui">
          {rename.error.message}
        </p>
      )}
    </>
  );
}
