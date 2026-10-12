import { createContext, useContext } from "react";

/** What components may know about the page being rendered. Provided by the renderer and the editor; never fetched during render. */
export type RenderEnv = {
  /** Page ID to URL. */
  urls: Record<string, string>;
  /** `YYYY-MM-DD` of the build, so "new until" dates are decided by the render, not by the clock of whoever views it. */
  today: string;
};

export const RenderEnvContext = createContext<RenderEnv>({ urls: {}, today: "" });

export const useRenderEnv = () => useContext(RenderEnvContext);
