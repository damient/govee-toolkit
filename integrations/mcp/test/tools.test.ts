// The tool list, compared to a snapshot. Update the snapshot with
// `npm test -- --test-update-snapshots` when a tool changes on purpose.

import assert from "node:assert/strict";
import { before, test } from "node:test";

import type { Tool } from "@modelcontextprotocol/client";

import { connect } from "./helpers.ts";
import { vocabulary } from "../src/catalog.ts";
import { docTopics, readApi } from "../src/data.ts";

const api = readApi();
let tools: Tool[];

before(async () => {
  const client = await connect();
  ({ tools } = await client.listTools());
});

// An enum read at startup follows the catalog and the docs, so the snapshot
// names its source. An enum that matches no source shows in the diff.
const sources = new Map<string, string>(
  Object.entries({
    modes: vocabulary.modes,
    capabilities: vocabulary.capabilities,
    support: vocabulary.support,
    personalities: vocabulary.personalities,
    roles: Object.keys(api.roles).toSorted(),
    languages: Object.keys(api.methods).toSorted(),
    topics: docTopics(),
  }).map(([source, values]) => [JSON.stringify(values), `<${source}>`]),
);

function withSources(value: unknown): unknown {
  if (Array.isArray(value)) return value.map((item) => withSources(item));
  if (typeof value !== "object" || value === null) return value;
  return Object.fromEntries(
    Object.entries(value).map(([key, item]) => [
      key,
      key === "enum" ? (sources.get(JSON.stringify(item)) ?? item) : withSources(item),
    ]),
  );
}

test("the tools and their schemas match the snapshot", (t) => {
  const shapes = tools
    .map(({ name, annotations, inputSchema, outputSchema }) => ({ name, annotations, inputSchema, outputSchema }))
    .toSorted((a, b) => a.name.localeCompare(b.name));
  t.assert.snapshot(withSources(shapes));
});

test("every tool carries a title, a description and an output schema", () => {
  for (const tool of tools) {
    assert.ok((tool.title ?? "") !== "", `${tool.name} has no title`);
    assert.ok((tool.description ?? "") !== "", `${tool.name} has no description`);
    assert.ok(tool.outputSchema, `${tool.name} has no output schema`);
  }
});

test("a read tool is read only, and a control tool that sends is not", () => {
  const hints = new Map(tools.map((tool) => [tool.name, tool.annotations]));
  for (const name of ["list_devices", "describe_device", "get_dmx_profile", "get_api", "read_doc"]) {
    assert.equal(hints.get(name)?.readOnlyHint, true, name);
    assert.equal(hints.get(name)?.openWorldHint, false, name);
  }
  for (const name of ["scan", "status", "set", "send", "identify"]) {
    assert.equal(hints.get(name)?.readOnlyHint, false, name);
    assert.equal(hints.get(name)?.destructiveHint, false, name);
    assert.equal(hints.get(name)?.openWorldHint, true, name);
  }
});
