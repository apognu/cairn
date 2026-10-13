import { RenderEnvContext } from "@yourdocs/components";
import { collectHeadings } from "./headings";
import { SiteLayout } from "./layout";
import { migrateDoc } from "./migrate";
import { renderNode } from "./render";
import { defaultContext, type RenderInput } from "./types";

/** A whole published page. The V8 renderer, the browser hydration and the editor's reader view all render this. */
export function Page({ doc: saved, context }: RenderInput) {
  const doc = migrateDoc(saved);
  const full = { ...defaultContext, ...context };
  const { list, byNode } = collectHeadings(doc);

  return (
    <RenderEnvContext.Provider
      value={{ urls: full.urls, pages: full.pages, variables: {}, snippets: {}, editing: false, today: full.today }}
    >
      <SiteLayout context={full} headings={list}>
        {renderNode(doc, { urls: full.urls, terms: full.terms, assets: full.assets, anchors: byNode }, 0)}
      </SiteLayout>
    </RenderEnvContext.Provider>
  );
}
