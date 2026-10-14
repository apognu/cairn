import { spawnSync } from "node:child_process";
import { existsSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { propsSchema, SDK_VERSION } from "@cairn/sdk";
import { testRender } from "./check";
import { BuildError, type Collected, collect } from "./collect";

const PACKAGES = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const WORKDIR = resolve(PACKAGES, "builder/.work");

/** The module `@cairn/components/custom` becomes: the deployer's components and slots, imported from the copy of their folder. */
function customModule(found: Collected): string {
  const imports = [...found.components.map((c) => c.file), ...found.slots.map((s) => s.file)].map(
    (file, i) => `import m${i} from "./components/${file}";`,
  );
  const components = found.components.map((_, i) => `m${i}`);
  const slots = found.slots.map((s, i) => `${s.definition.name}: m${found.components.length + i}`);

  return [
    `import type { AnyComponent, SlotDefinition, SlotName } from "@cairn/sdk";`,
    ...imports,
    `export const customComponents = [${components.join(", ")}] as unknown as AnyComponent[];`,
    `export const customSlots = { ${slots.join(", ")} } as Partial<Record<SlotName, SlotDefinition>>;`,
    "",
  ].join("\n");
}

function vite(pkg: string, config: string, custom: string, out: string): void {
  const run = spawnSync("bunx", ["vite", "build", "--config", config, "--logLevel", "warn"], {
    cwd: join(PACKAGES, pkg),
    env: { ...process.env, NODE_ENV: "production", CAIRN_CUSTOM: custom, CAIRN_OUT: out },
    encoding: "utf8",
  });

  if (run.status !== 0) throw new BuildError(`the ${pkg} build failed:\n${run.stdout}${run.stderr}`);
}

/**
 * Builds the browser bundle (the editor and the published pages' scripts and styles), `ssr.js` for V8 and a manifest from a folder of
 * components and slots. A component that cannot render fails the build, naming it.
 */
export async function buildComponents(dir: string, out: string, log: (line: string) => void = () => {}): Promise<void> {
  const output = resolve(out);
  // The server render empties the output folder first: only an empty folder or bundles built before may be there.
  if (existsSync(output) && readdirSync(output).length > 0 && !existsSync(join(output, "manifest.json")))
    throw new BuildError(
      `${out} is not empty and holds no bundles built before; choose an empty or new folder for --out`,
    );

  const found = await collect(resolve(dir), WORKDIR);
  log(`${found.components.length} component(s), ${found.slots.length} slot(s)`);

  const custom = join(WORKDIR, "custom.ts");
  writeFileSync(custom, customModule(found));

  // The server render first: a broken component stops the build before the slower browser bundles.
  vite("renderer", "vite.config.ts", custom, output);
  for (const file of readdirSync(output)) if (file.endsWith(".css")) rmSync(join(output, file));
  testRender(await Bun.file(join(output, "ssr.js")).text(), found);
  log("every component renders");

  vite("renderer", "vite.client.config.ts", custom, output);
  vite("editor", "vite.config.ts", custom, join(output, "editor"));

  const manifest = {
    sdk_version: SDK_VERSION,
    components: found.components.map(({ definition: d }) => ({
      name: d.name,
      title: d.title,
      description: d.description ?? null,
      version: d.version,
      interactive: d.interactive ?? false,
      schema: propsSchema(d),
    })),
    slots: found.slots.map((s) => s.definition.name),
  };
  writeFileSync(join(output, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
  log(`bundles in ${relative(process.cwd(), output) || "."}`);
}
