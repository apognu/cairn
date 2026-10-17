import { useEffect, useId, useRef, useState } from "react";
import { useCan, useMarkReviewed, useStalePages } from "./queries";
import { reviewNote } from "./Sidebar";
import type { Page } from "./tree";
import * as ui from "./ui";

/** Today plus six months, `YYYY-MM-DD` in local time: the next review date offered by default. */
function inSixMonths(): string {
  const date = new Date();
  date.setMonth(date.getMonth() + 6);
  return date.toLocaleDateString("sv-SE");
}

/** Over a page whose review date has come: since when, who owns it, and for editors a way to mark it reviewed. */
export function ReviewBanner({ page }: { page: Page }) {
  const id = useId();
  const due = useStalePages().data?.find((s) => s.page_id === page.id);
  const canEdit = useCan("editor");
  const mark = useMarkReviewed(page.id);
  const [asking, setAsking] = useState(false);
  const [next, setNext] = useState(inSixMonths);
  const form = useRef<HTMLFormElement>(null);
  const field = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!asking) return;
    field.current?.focus();
    const close = (event: Event) => {
      if (event instanceof KeyboardEvent ? event.key === "Escape" : !form.current?.contains(event.target as Node))
        setAsking(false);
    };
    document.addEventListener("keydown", close);
    document.addEventListener("pointerdown", close);
    return () => {
      document.removeEventListener("keydown", close);
      document.removeEventListener("pointerdown", close);
    };
  }, [asking]);

  if (!due) return null;

  return (
    <output
      className="flex flex-wrap items-center gap-x-3.5 gap-y-2 py-2.5 px-6 border-b border-border bg-accent-soft text-ui [@media(max-width:700px)]:px-4"
      data-testid="review-banner"
    >
      <span className="flex-[1_1_260px] min-w-0">{reviewNote(due)}</span>
      {canEdit &&
        (asking ? (
          <form
            ref={form}
            className="flex flex-wrap items-center gap-2 ml-auto"
            onSubmit={(e) => {
              e.preventDefault();
              mark.mutate(next, { onSuccess: () => setAsking(false) });
            }}
          >
            <label htmlFor={`${id}-next`} className="text-help">
              Next review
            </label>
            <input
              id={`${id}-next`}
              ref={field}
              type="date"
              className="w-40 h-8.5 px-2.5 border border-input rounded-md bg-bg"
              value={next}
              required
              onChange={(e) => setNext(e.target.value)}
            />
            <button type="submit" className={ui.primary} disabled={mark.isPending || !next}>
              Mark as reviewed
            </button>
            {mark.error && (
              <span role="alert" className="basis-full text-danger text-help">
                {mark.error.message}
              </span>
            )}
          </form>
        ) : (
          <button type="button" className={`${ui.secondary} ml-auto`} onClick={() => setAsking(true)}>
            Mark as reviewed
          </button>
        ))}
    </output>
  );
}
