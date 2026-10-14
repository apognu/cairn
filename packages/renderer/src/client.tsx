import { type IconNode, registerIcons } from "@yourdocs/components";
import { createRoot, hydrateRoot } from "react-dom/client";
import { Page } from "./page";
import type { RenderInput } from "./types";
import "./site.css";

const root = document.getElementById("root");
const source = document.getElementById("doc");

if (!root || !source?.textContent) throw new Error("missing #root or #doc in the page");

// The icons the server drew, listed in the page: the browser bundle carries no icon collection of its own.
const icons = document.getElementById("icons")?.textContent;
if (icons) registerIcons(JSON.parse(icons) as Record<string, IconNode>);

const page = <Page {...(JSON.parse(source.textContent) as RenderInput)} />;

// An empty root means the page was served without a server render (the parity check's reference).
if (root.hasChildNodes()) hydrateRoot(root, page);
else createRoot(root).render(page);
