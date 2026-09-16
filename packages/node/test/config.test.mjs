// The configuration. Nothing here starts a transport.

import assert from "node:assert/strict";
import { join } from "node:path";
import test from "node:test";

import { here } from "./helpers.mjs";
import { Config } from "../dist/index.js";

test("the default configuration enables lan alone", () => {
  const config = new Config();
  assert.deepEqual(config.defaultModes, ["lan"]);
  assert.deepEqual(config.devices, []);
});

test("the default configuration carries a stream rate", () => {
  assert.ok(new Config().streamFallbackHz > 0);
});

test("a missing file is the default configuration", () => {
  const loaded = Config.loadFrom(join(here, "no-such-file.yaml"));
  assert.deepEqual(loaded.defaultModes, new Config().defaultModes);
  assert.deepEqual(loaded.devices, []);
});

test("load without a file is the default configuration", () => {
  assert.deepEqual(Config.load().defaultModes, ["lan"]);
});

test("the whole configuration reads as an object", () => {
  const config = new Config().toJSON();
  assert.deepEqual(config.defaults.modes, ["lan"]);
  assert.deepEqual(config.devices, {});
  assert.ok(config.stream.fallback_hz > 0);
});

test("the configuration object carries no credential", () => {
  assert.equal(new Config().toJSON().env, undefined);
});
