import { createRoot, hydrateRoot } from "react-dom/client";
import { Page } from "./page";
import type { RenderInput } from "./types";
import "./site.css";

const root = document.getElementById("root");
const source = document.getElementById("doc");

if (!root || !source?.textContent) throw new Error("missing #root or #doc in the page");

const page = <Page {...(JSON.parse(source.textContent) as RenderInput)} />;

// An empty root means the page was served without a server render (the parity check's reference).
if (root.hasChildNodes()) hydrateRoot(root, page);
else createRoot(root).render(page);
