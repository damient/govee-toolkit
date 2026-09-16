// What the package exports, and what it says about itself.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import test from "node:test";

import { packageRoot, VERSION_PATTERN } from "./helpers.mjs";
import * as govee from "../dist/index.js";

const EXPORTED = [
  "Govee",
  "Config",
  "Catalog",
  "Device",
  "DeviceHandle",
  "DeviceStatus",
  "Health",
  "Reply",
  "Served",
  "SegmentStream",
  "EventStream",
  "StatusStream",
];

test("the package exports the public API", () => {
  for (const name of EXPORTED) {
    assert.equal(typeof govee[name], "function", `${name} is missing`);
  }
});

test("the version is a release number", () => {
  assert.match(govee.VERSION, VERSION_PATTERN);
});

test("the version is the one the manifest declares", () => {
  const manifest = JSON.parse(
    readFileSync(join(packageRoot, "package.json"), "utf8"),
  );
  assert.equal(govee.VERSION, manifest.version);
});

test("the core version is the crate the binding was built from", () => {
  assert.match(govee.CORE_VERSION, VERSION_PATTERN);
});

test("the modes are the names the core knows", () => {
  assert.ok(govee.MODES.includes("lan"));
  assert.equal(new Set(govee.MODES).size, govee.MODES.length);
  assert.ok(govee.MODES.every((name) => name === name.toLowerCase()));
});

test("the mode names cannot be written to", () => {
  assert.ok(Object.isFrozen(govee.MODES));
});
