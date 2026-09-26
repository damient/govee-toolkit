// The tools serve the same content: many clients do not let the model read a resource.

import { type McpServer, ResourceTemplate } from "@modelcontextprotocol/server";

import { allSkus, record } from "./catalog.ts";
import { docTopics, readDoc } from "./data.ts";

export function registerResources(server: McpServer): void {
  for (const topic of docTopics()) {
    const uri = `gtk://docs/${topic}`;
    server.registerResource(topic, uri, { title: `docs/${topic}.md`, mimeType: "text/markdown" }, () => ({
      contents: [{ uri, mimeType: "text/markdown", text: readDoc(topic) }],
    }));
  }

  const devices = new ResourceTemplate("gtk://devices/{sku}", {
    list: () => ({
      resources: allSkus().map((sku) => ({ uri: `gtk://devices/${sku}`, name: sku, mimeType: "application/json" })),
    }),
    complete: { sku: (value) => allSkus().filter((sku) => sku.startsWith(value.toUpperCase())) },
  });
  server.registerResource(
    "device",
    devices,
    { title: "One device record, as `describe_device` returns it", mimeType: "application/json" },
    (uri, { sku }) => {
      const text = JSON.stringify(record(String(sku)));
      return { contents: [{ uri: uri.href, mimeType: "application/json", text }] };
    },
  );
}
