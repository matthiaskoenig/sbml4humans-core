import type { Report } from "./report.js";
import type { Input, WasmSource } from "./index.mjs";
export interface SynchronousReader {
  report(source: Input): Report;
  release(): void;
}
export function compileReader(source?: WasmSource): Promise<WebAssembly.Module>;
export function instantiateReader(source?: WasmSource): Promise<SynchronousReader>;
