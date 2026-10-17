import { pageTarget } from "@cairn/sdk";
import { createContext, type ReactNode, useContext } from "react";

/** What components may know about the page being rendered. Provided by the renderer and the editor; never fetched during render. */
export type RenderEnv = {
  /** Page ID to URL. */
  urls: Record<string, string>;
  /** Page ID to what other pages show about it: link cards read the title and description from here. */
  pages: Record<string, { title: string; description?: string | null; sections?: Record<string, string> }>;
  /** Variable name to its value, for the editor; published pages have them written in already. */
  variables: Record<string, string>;
  /** Snippet name to its drawn content, for the editor. */
  snippets: Record<string, ReactNode>;
  /** True in the editor, where a component with nothing chosen yet shows a prompt instead of nothing. */
  editing: boolean;
  /** `YYYY-MM-DD` of the build, so "new until" dates are decided by the render, not by the clock of whoever views it. */
  today: string;
};

export const RenderEnvContext = createContext<RenderEnv>({
  urls: {},
  pages: {},
  variables: {},
  snippets: {},
  editing: false,
  today: "",
});

export const useRenderEnv = () => useContext(RenderEnvContext);

/** Where a page field leads: the page's URL, followed by the operation's anchor when it names one; undefined while the page has no URL. */
export function pageHref(urls: Record<string, string>, value: string): string | undefined {
  const { id, anchor } = pageTarget(value);
  const url = urls[id];
  return url && anchor ? `${url}#${anchor}` : url;
}
