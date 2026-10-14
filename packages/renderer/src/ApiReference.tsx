import type { ApiContext, ApiOperation } from "@cairn/sdk";
import { useEffect, useRef, useState } from "react";

type ScalarInstance = { destroy: () => void };
type ScalarGlobal = { createApiReference: (target: HTMLElement, config: Record<string, unknown>) => ScalarInstance };

/** Where Scalar puts an operation in the address: `#tag/<tag in lower case, dashes for spaces>/<METHOD><path>`. */
function scalarAnchor(operation: ApiOperation): string {
  const tag = (operation.tags[0] ?? "default").trim().toLowerCase().replace(/\s+/g, "-");
  return `tag/${tag}/${operation.method}${operation.path}`;
}

// One script per page, however many references mount.
let scalarScript: Promise<ScalarGlobal> | undefined;

function loadScalar(url: string): Promise<ScalarGlobal> {
  scalarScript ??= new Promise((resolve, reject) => {
    const script = document.createElement("script");
    script.src = url;
    script.onload = () => {
      const scalar = (window as unknown as { Scalar?: ScalarGlobal }).Scalar;
      if (scalar) resolve(scalar);
      else reject(new Error("the API reference script did not start"));
    };
    script.onerror = () => {
      scalarScript = undefined;
      reject(new Error(`cannot load ${url}`));
    };
    document.head.append(script);
  });

  return scalarScript;
}

/** Whether the page shows dark colors now: the visitor's choice in the header toggle, else the system's. */
function isDark(): boolean {
  const chosen = document.documentElement.dataset.theme;
  return chosen ? chosen === "dark" : window.matchMedia("(prefers-color-scheme: dark)").matches;
}

/** Scalar's colors and fonts come from the site's own custom properties, so the theme, custom CSS and dark mode apply to it as to the rest. */
const SCALAR_CSS = `
.scalar-app, .light-mode, .dark-mode {
  --scalar-background-1: var(--bg);
  --scalar-background-2: var(--bg-side);
  --scalar-background-3: var(--hover);
  --scalar-background-accent: color-mix(in srgb, var(--accent) 12%, var(--bg));
  --scalar-color-1: var(--text);
  --scalar-color-2: var(--text-body);
  --scalar-color-3: var(--muted);
  --scalar-color-accent: var(--accent);
  --scalar-border-color: var(--border);
  --scalar-link-color: var(--link);
  --scalar-link-color-hover: var(--link);
  --scalar-code-background: var(--code-bg);
  --scalar-font: var(--font-sans);
  --scalar-font-code: var(--font-mono);
  --scalar-custom-header-height: 52px;
}
`;

function configuration(api: ApiContext, compact: boolean): Record<string, unknown> {
  return {
    url: api.specUrl,
    theme: "none",
    layout: "modern",
    withDefaultFonts: false,
    showSidebar: !compact,
    hideSearch: true,
    hideDarkModeToggle: true,
    showDeveloperTools: "never",
    documentDownloadType: "none",
    forceDarkModeState: isDark() ? "dark" : "light",
    hideModels: api.display.hideModels,
    hideClientButton: !api.display.requestClient,
    hideTestRequestButton: !api.display.requestClient,
    telemetry: false,
    agent: { disabled: true },
    mcp: { disabled: true },
    customCss: SCALAR_CSS,
  };
}

/**
 * An API reference: Scalar, mounted once the page runs. JavaScript is required; the server render holds only the mount point. `compact`
 * leaves out Scalar's own sidebar, for a preview in a narrow column.
 */
export function ApiReference({ api, compact = false }: { api: ApiContext; compact?: boolean }) {
  const target = useRef<HTMLDivElement>(null);
  // The search-only block goes once Scalar is there: its headings carry the same ids as Scalar's own sections.
  const [mounted, setMounted] = useState(false);
  const { specUrl, scalarUrl } = api;

  // biome-ignore lint/correctness/useExhaustiveDependencies: the instance is made once per spec
  useEffect(() => {
    const element = target.current;
    if (!element || !scalarUrl) return;

    let instance: ScalarInstance | undefined;
    let cancelled = false;

    loadScalar(scalarUrl)
      .then((scalar) => {
        if (cancelled) return;
        instance = scalar.createApiReference(element, configuration(api, compact));
        setMounted(true);
      })
      .catch(() => {
        element.textContent = "The API reference could not be loaded.";
      });

    // Scalar reads its color mode once, when it starts, and its own toggle is hidden: the site's toggle and the system setting decide, so
    // the classes it puts on the body follow them from here. Its colors already come from the site's custom properties.
    const sync = () => {
      const dark = isDark();
      // Only when it differs, or the change of class would call this again.
      if (document.body.classList.contains("dark-mode") !== dark) document.body.classList.toggle("dark-mode", dark);
      if (document.body.classList.contains("light-mode") === dark) document.body.classList.toggle("light-mode", !dark);
    };
    const observer = new MutationObserver(sync);
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
    // Scalar puts its own mode back when the system setting changes, so the body is watched as well.
    observer.observe(document.body, { attributes: true, attributeFilter: ["class"] });
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    media.addEventListener("change", sync);

    return () => {
      cancelled = true;
      observer.disconnect();
      media.removeEventListener("change", sync);
      instance?.destroy();
    };
  }, [specUrl, scalarUrl]);

  return (
    <div className="api-reference">
      <noscript>This API reference needs JavaScript.</noscript>
      {/* Not shown to anyone: it is what the site search reads, from the merged spec. Its ids are Scalar's, so a result lands on the operation. */}
      {!mounted && (
        <div hidden>
          {api.operations.map((operation) => (
            <section key={`${operation.method} ${operation.path}`}>
              <h2 id={scalarAnchor(operation)}>
                {operation.method} {operation.path}
              </h2>
              {operation.summary && <p>{operation.summary}</p>}
              {operation.description && <p>{operation.description}</p>}
            </section>
          ))}
        </div>
      )}
      <div ref={target} className="api-mount" />
    </div>
  );
}
