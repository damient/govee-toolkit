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

test("a SKU describes as govee describe --json prints it", () => {
  const sku = catalog.skus()[0];
  const record = catalog.describe(sku);
  assert.equal(record.sku, catalog.device(sku).sku);
  for (const key of ["modes", "commands"]) {
    assert.ok(key in record, key);
  }
});

test("a SKU gives its DMX channel tables", () => {
  for (const sku of catalog.skus()) {
    const entry = catalog.dmx(sku);
    assert.ok(Array.isArray(entry.personalities), sku);
  }
});

test("describe and dmx refuse an unknown SKU", () => {
  for (const method of ["describe", "dmx"]) {
    assert.throws(() => catalog[method]("H0000"), (error) => {
      assert.equal(error.code, "unknown_sku");
      assert.equal(error.name, "CodecError");
      return true;
    }, method);
  }
});

test("the capability list holds what every device declares, sorted", () => {
  const names = catalog.capabilities();
  assert.deepEqual(names, [...names].sort());
  for (const sku of catalog.skus()) {
    for (const name of catalog.describe(sku).capabilities) {
      assert.ok(names.includes(name), `${sku}: ${name}`);
    }
  }
});
