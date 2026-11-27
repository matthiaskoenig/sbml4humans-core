import { homedir } from "node:os";
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { gzipSync } from "node:zlib";

const root = fileURLToPath(new URL("../", import.meta.url));
execFileSync("cargo", ["build", "-p", "sbml4humans-wasm", "--profile", "wasm", "--locked", "--target", "wasm32-unknown-unknown"], { cwd: root, stdio: "inherit", env: { ...process.env, RUSTFLAGS: `${process.env.RUSTFLAGS || ""} --remap-path-prefix=${root}=/sbml4humans-core/ --remap-path-prefix=${process.env.CARGO_HOME || `${homedir()}/.cargo`}=/cargo` } });
const runtime = new URL("./runtime/", import.meta.url);
mkdirSync(runtime, { recursive: true });
const target = new URL("sbml4humans-core.wasm", runtime);
copyFileSync(new URL("../target/wasm32-unknown-unknown/wasm/sbml4humans_wasm.wasm", import.meta.url), target);
const bytes = readFileSync(target);
const module = await WebAssembly.compile(bytes);
if (WebAssembly.Module.imports(module).length) throw new Error("The WASM report module must have no imports.");
console.log(`WASM report parser: ${bytes.length} bytes, ${gzipSync(bytes).length} gzipped; no imports.`);
