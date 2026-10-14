import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { type IconNode, registerIcons } from "@yourdocs/components";
import icons from "lucide-static/icon-nodes.json";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "@yourdocs/renderer/site.css";
import "./styles.css";

// The editor draws any icon of the collection: in components on the canvas and in the icon picker.
registerIcons(icons as unknown as Record<string, IconNode>);

const client = new QueryClient();
const root = document.getElementById("root");
if (!root) throw new Error("#root is missing");

createRoot(root).render(
  <StrictMode>
    <QueryClientProvider client={client}>
      <App />
    </QueryClientProvider>
  </StrictMode>,
);
