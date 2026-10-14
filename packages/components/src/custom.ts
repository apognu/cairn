import type { AnyComponent, SlotDefinition, SlotName } from "@cairn/sdk";

// Empty here. `build-components` points `@cairn/components/custom` at a generated module that lists the deployer's components and slots,
// so the editor, the browser bundle and `ssr.js` all see the same ones.
export const customComponents: AnyComponent[] = [];
export const customSlots: Partial<Record<SlotName, SlotDefinition>> = {};
