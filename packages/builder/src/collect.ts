import { cpSync, mkdirSync, readdirSync, rmSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { pathToFileURL } from "node:url";
import { componentsByName } from "@cairn/components";
import { type AnyComponent, propsSchema, SLOT_NAMES, type SlotDefinition } from "@cairn/sdk";

/** A problem with the deployer's folder, worded for them: the file and what to change. */
export class BuildError extends Error {}

export type Collected = {
  /** The copy of the folder, inside the workspace so its imports resolve like the built-in components' do. */
  copy: string;
  components: { file: string; definition: AnyComponent }[];
  slots: { file: string; definition: SlotDefinition }[];
};

const SOURCE = /\.(tsx?|jsx?)$/;
const NAME = /^[a-z0-9][a-z0-9-]*\/[a-z0-9][a-z0-9-]*$/;

// Sorted: the directory's own order depends on the filesystem, and the bundles and the slash menu should not.
function sourceFiles(dir: string, base = dir): string[] {
  return readdirSync(dir)
    .sort()
    .flatMap((entry) => {
      const path = join(dir, entry);
      if (entry === "node_modules" || entry.startsWith(".")) return [];
      if (statSync(path).isDirectory()) return sourceFiles(path, base);

      // `_helpers.tsx`, tests and type declarations are not entry points.
      const skip = entry.startsWith("_") || entry.endsWith(".d.ts") || /\.(test|spec)\./.test(entry);
      return SOURCE.test(entry) && !skip ? [relative(base, path)] : [];
    });
}

// Components import their styles; the check below only loads the modules, so a stylesheet is an empty one there. Vite bundles the real one.
let stubbed = false;
function stubStyles() {
  if (stubbed) return;
  stubbed = true;
  Bun.plugin({
    name: "styles",
    setup: (build) => {
      build.onLoad({ filter: /\.css$/ }, () => ({ contents: "", loader: "js" }));
    },
  });
}

function check(file: string, definition: AnyComponent): void {
  const problem = (message: string) => new BuildError(`${file}: ${definition.name || "component"} ${message}`);

  if (!NAME.test(definition.name))
    throw problem(`needs a namespaced name like "acme/link-tabs" (lowercase letters, digits and dashes)`);
  if (componentsByName.has(definition.name)) throw problem("is the name of a built-in component");
  if (!Number.isInteger(definition.version) || definition.version < 1)
    throw problem("needs a whole version number from 1");
  if (!definition.title) throw problem("needs a title");
  if (typeof definition.render !== "function") throw problem("needs a render function");

  for (const from of Object.keys(definition.migrate ?? {})) {
    if (Number(from) >= definition.version)
      throw problem(`has a migrate step from version ${from}, but it is version ${definition.version} itself`);
  }

  try {
    propsSchema(definition);
  } catch (error) {
    throw problem(`has props that cannot become a settings form: ${(error as Error).message}`);
  }
}

/** Copies the folder into the workspace and loads what every file exports by default: a component (`defineComponent`) or a slot (`defineSlot`). */
export async function collect(dir: string, workdir: string): Promise<Collected> {
  const copy = join(workdir, "components");
  rmSync(workdir, { recursive: true, force: true });
  mkdirSync(workdir, { recursive: true });
  cpSync(dir, copy, { recursive: true, filter: (path) => !path.split("/").includes("node_modules") });

  stubStyles();
  const result: Collected = { copy, components: [], slots: [] };

  for (const file of sourceFiles(copy)) {
    let exported: unknown;
    try {
      exported = ((await import(pathToFileURL(join(copy, file)).href)) as { default?: unknown }).default;
    } catch (error) {
      throw new BuildError(`${file}: cannot be loaded: ${(error as Error).message}`);
    }
    if (!exported || typeof exported !== "object")
      throw new BuildError(`${file}: the default export must be defineComponent(…) or defineSlot(…)`);

    if ((exported as SlotDefinition).kind === "slot") {
      const slot = exported as SlotDefinition;
      if (!SLOT_NAMES.includes(slot.name))
        throw new BuildError(`${file}: "${slot.name}" is not a slot (${SLOT_NAMES.join(", ")})`);

      const other = result.slots.find((s) => s.definition.name === slot.name);
      if (other) throw new BuildError(`${file}: the ${slot.name} slot is already defined in ${other.file}`);
      result.slots.push({ file, definition: slot });
    } else {
      const definition = exported as AnyComponent;
      check(file, definition);

      const other = result.components.find((c) => c.definition.name === definition.name);
      if (other) throw new BuildError(`${file}: ${definition.name} is already defined in ${other.file}`);
      result.components.push({ file, definition });
    }
  }

  return result;
}
