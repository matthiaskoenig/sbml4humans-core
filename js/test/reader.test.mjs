import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";
import { gzipSync } from "node:zlib";
import { createReader } from "../index.mjs";
import { compileReader, instantiateReader } from "../wasm.mjs";

const source = await readFile(new URL("../../testdata/level2_graph_units.xml", import.meta.url));
const oracle = JSON.parse(await readFile(new URL("../../testdata/oracles/level2_graph_units.json", import.meta.url), "utf8"));
const module = await compileReader();

test("the real WASM artifact has no host imports", () => {
  assert.deepEqual(WebAssembly.Module.imports(module), []);
});
test("WASM preserves every report field and survives invalid input", async () => {
  const reader = await instantiateReader(module);
  assert.throws(() => reader.report("not XML"), { kind: "malformed" });
  assert.deepEqual(reader.report(source), oracle);
  reader.release();
  assert.deepEqual(reader.report(source), oracle);
});
test("worker parsing preserves caller buffers and does not retain a failed request", async () => {
  const reader = await createReader({ source: module });
  try {
    await assert.rejects(reader.report("not XML"), { kind: "malformed" });
    const input = new Uint8Array(source);
    assert.deepEqual(await reader.report(input), oracle);
    assert.equal(input.byteLength, source.byteLength);
  } finally { reader.dispose(); }
});
test("abort replaces the active worker and resumes other queued reports", async () => {
  const reader = await createReader({ source: module });
  try {
    const controller = new AbortController();
    const first = reader.report(source, { signal: controller.signal });
    const rejected = assert.rejects(first, { name: "AbortError" });
    const next = reader.report(source);
    controller.abort();
    await rejected;
    assert.deepEqual(await next, oracle);
  } finally { reader.dispose(); }
});
test("cancelled queued requests and disposal settle every outstanding promise", async () => {
  const reader = await createReader({ source: module });
  const first = reader.report(source);
  const firstRejected = assert.rejects(first, { name: "AbortError" });
  const controller = new AbortController();
  const queued = reader.report(source, { signal: controller.signal });
  const queuedRejected = assert.rejects(queued, { name: "AbortError" });
  controller.abort();
  reader.dispose();
  await Promise.all([firstRejected, queuedRejected]);
  await assert.rejects(reader.report(source), /disposed/);
});
test("an incompatible artifact rejects creation instead of leaving requests pending", async () => {
  const empty = await WebAssembly.compile(new Uint8Array([0,97,115,109,1,0,0,0]));
  await assert.rejects(createReader({ source: empty }), /missing report_alloc/);
});

test("every standalone oracle matches through the real WASM worker, plain and gzip", async () => {
  const manifest = JSON.parse(await readFile(new URL("../../testdata/oracles/MANIFEST.json", import.meta.url)));
  const reader = await createReader({ source: module });
  try {
    for (const entry of manifest.files) {
      const xml = await readFile(new URL(`../../testdata/${entry.source}`, import.meta.url));
      const expected = JSON.parse(await readFile(new URL(`../../testdata/oracles/${entry.fixture}`, import.meta.url)));
      assert.deepEqual(await reader.report(xml), expected, entry.source);
      assert.deepEqual(await reader.report(gzipSync(xml)), expected, `${entry.source} (gzip)`);
    }
    await assert.rejects(reader.report(new Uint8Array([31, 139, 8])), { kind: "malformed" });
    assert.deepEqual(await reader.report(source), oracle);
  } finally { reader.dispose(); }
});
