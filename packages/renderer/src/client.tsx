import { createRoot, hydrateRoot } from "react-dom/client";
import { Doc, type DocNode } from "./doc";

const root = document.getElementById("root");
const source = document.getElementById("doc");

if (!root || !source?.textContent) throw new Error("missing #root or #doc in the page");

const page = <Doc doc={JSON.parse(source.textContent) as DocNode} />;

// An empty root means the page was served without a server render (the parity check's reference).
if (root.hasChildNodes()) hydrateRoot(root, page);
else createRoot(root).render(page);
