// Every read tool and resource, called on the embedded catalog. None of them
// starts the SDK.

import assert from "node:assert/strict";
import { after, before, test } from "node:test";

import type { Client } from "@modelcontextprotocol/client";
import { Catalog } from "govee-toolkit";
import { z } from "zod";

import { connect, failureCode, structured } from "./helpers.ts";
import { docTopics, readApi } from "../src/data.ts";

const skus: string[] = Catalog.embedded().skus();
const api = readApi();
let client: Client;

before(async () => {
  client = await connect();
});

after(async () => {
  await client.close();
});

const rowsShape = z.object({
  devices: z.array(z.object({ sku: z.string(), support: z.record(z.string(), z.string()) })),
});
const personalities = z.object({ personalities: z.array(z.looseObject({ personality: z.string() })) });
const apiShape = z.object({
  roles: z.record(z.string(), z.record(z.string(), z.array(z.string()))),
  methods: z.array(z.string()).optional(),
});

async function rows(filter: Record<string, unknown> = {}): Promise<z.infer<typeof rowsShape>["devices"]> {
  return (await structured(client, "list_devices", filter, rowsShape)).devices;
}

test("list_devices returns every SKU of the catalog, once", async () => {
  const listed = (await rows()).map((row) => row.sku);
  assert.deepEqual(listed.toSorted(), skus.toSorted());
});

test("list_devices keeps the rows whose support on the mode matches", async () => {
  const [first] = await rows();
  assert.ok(first);
  const [mode, support] = Object.entries(first.support)[0] ?? [];
  assert.ok(mode !== undefined && support !== undefined);
  const kept = await rows({ mode, support });
  assert.ok(kept.some((row) => row.sku === first.sku));
  assert.ok(kept.every((row) => row.support[mode] === support));
});

test("list_devices refuses `support` without `mode`", async () => {
  const [first] = await rows();
  const support = Object.values(first?.support ?? {})[0];
  assert.equal(await failureCode(client, "list_devices", { support }), "invalid_argument");
});

test("describe_device answers every SKU, with its DMX tables", async () => {
  const record = z.looseObject({ name: z.string(), dmx: personalities });
  await Promise.all(skus.map((sku) => structured(client, "describe_device", { sku }, record)));
});

test("describe_device fails with unknown_sku", async () => {
  assert.equal(await failureCode(client, "describe_device", { sku: "NOT-A-SKU" }), "unknown_sku");
});

test("get_dmx_profile answers every SKU, and each personality alone", async () => {
  const each = async (sku: string): Promise<void> => {
    const all = await structured(client, "get_dmx_profile", { sku }, personalities);
    const alone = await Promise.all(
      all.personalities.map(({ personality }) =>
        structured(client, "get_dmx_profile", { sku, personality }, personalities),
      ),
    );
    assert.deepEqual(
      alone.map((one) => one.personalities.map((p) => p.personality)),
      all.personalities.map((p) => [p.personality]),
    );
  };
  await Promise.all(skus.map((sku) => each(sku)));
});

test("get_dmx_profile fails with unknown_sku", async () => {
  assert.equal(await failureCode(client, "get_dmx_profile", { sku: "NOT-A-SKU" }), "unknown_sku");
});

test("get_api returns every role of api.json", async () => {
  const { roles } = await structured(client, "get_api", {}, apiShape);
  assert.deepEqual(Object.keys(roles).toSorted(), Object.keys(api.roles).toSorted());
});

test("get_api narrows to one role and one language", async () => {
  const [role] = Object.keys(api.roles);
  const [language] = Object.keys(api.methods);
  assert.ok(role !== undefined && language !== undefined);
  const answer = await structured(client, "get_api", { role, language }, apiShape);
  assert.deepEqual(Object.keys(answer.roles), [role]);
  assert.deepEqual(Object.keys(answer.roles[role] ?? {}), [language]);
  assert.deepEqual(answer.methods, api.methods[language]);
});

test("read_doc returns every page that the build copied", async () => {
  const topics = docTopics();
  assert.ok(topics.length > 0);
  const page = z.object({ topic: z.string(), text: z.string().min(1) });
  const pages = await Promise.all(topics.map((topic) => structured(client, "read_doc", { topic }, page)));
  assert.deepEqual(
    pages.map((p) => p.topic),
    topics,
  );
});

test("each docs page and each device record is a resource", async () => {
  const { resources } = await client.listResources();
  const uris = new Set(resources.map((resource) => resource.uri));
  for (const topic of docTopics()) assert.ok(uris.has(`gtk://docs/${topic}`), topic);
  for (const sku of skus) assert.ok(uris.has(`gtk://devices/${sku}`), sku);

  const [sku] = skus;
  const { contents } = await client.readResource({ uri: `gtk://devices/${sku}` });
  const [content] = contents;
  assert.ok(content && "text" in content);
  z.looseObject({ sku: z.string(), dmx: personalities }).parse(JSON.parse(content.text));
});
