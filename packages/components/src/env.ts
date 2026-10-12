import { createContext, useContext } from "react";

/** What components may know about the page being rendered. Provided by the renderer and the editor; never fetched during render. */
export type RenderEnv = {
  /** Page ID to URL. */
  urls: Record<string, string>;
  /** Page ID to what other pages show about it: link cards read the title and description from here. */
  pages: Record<string, { title: string; description?: string | null }>;
  /** True in the editor, where a component with nothing chosen yet shows a prompt instead of nothing. */
  editing: boolean;
  /** `YYYY-MM-DD` of the build, so "new until" dates are decided by the render, not by the clock of whoever views it. */
  today: string;
};

export const RenderEnvContext = createContext<RenderEnv>({ urls: {}, pages: {}, editing: false, today: "" });

export const useRenderEnv = () => useContext(RenderEnvContext);
