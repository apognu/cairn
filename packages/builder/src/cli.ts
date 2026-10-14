#!/usr/bin/env bun
import { buildComponents } from "./build";
import { BuildError } from "./collect";

const USAGE = "usage: cairn build-components <components-folder> --out <bundles-folder>";

const [command, dir, flag, out] = process.argv.slice(2);

if (command !== "build-components" || !dir || flag !== "--out" || !out) {
  process.stderr.write(`${USAGE}\n`);
  process.exit(2);
}

try {
  await buildComponents(dir, out, (line) => process.stdout.write(`${line}\n`));
} catch (error) {
  process.stderr.write(`build-components failed: ${error instanceof BuildError ? error.message : String(error)}\n`);
  process.exit(1);
}
