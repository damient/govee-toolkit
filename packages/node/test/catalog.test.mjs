// The embedded catalog. It reads no hardware and no file.

import assert from "node:assert/strict";
import { existsSync, readdirSync } from "node:fs";
import { join } from "node:path";
import test from "node:test";

import { repositoryRoot } from "./helpers.mjs";
import { Catalog } from "../dist/index.js";

const catalog = Catalog.embedded();

test("the catalog carries devices", () => {
  assert.ok(catalog.skus().length > 0);
  assert.ok(catalog.size > 0);
});

test("every listed SKU resolves", () => {
  for (const sku of catalog.skus()) {
    assert.ok(catalog.has(sku), sku);
  }
});

test("a device file comes back whole", () => {
  const device = catalog.device(catalog.skus()[0]);
  assert.equal(typeof device, "object");
  assert.ok(device.sku);
  assert.ok("commands" in device);
  assert.ok("modes" in device);
});

test("every device file of the checkout is embedded", (t) => {
  const devices = join(repositoryRoot, "devices");
  if (!existsSync(devices)) {
    t.skip("no checkout: the device files are not here to compare against");
    return;
  }
  const onDisk = readdirSync(devices)
    .filter((name) => name.endsWith(".yaml") && name !== "schema.yaml")
    .map((name) => name.replace(/\.yaml$/, ""));
  assert.ok(onDisk.length > 0);
  const embedded = new Set(catalog.skus());
  for (const sku of onDisk) {
    assert.ok(embedded.has(sku), sku);
  }
});

test("an unknown SKU is refused", () => {
  assert.equal(catalog.has("H0000"), false);
  assert.throws(() => catalog.device("H0000"), (error) => {
    assert.equal(error.code, "unknown_sku");
    assert.equal(error.name, "CodecError");
    return true;
  });
});
