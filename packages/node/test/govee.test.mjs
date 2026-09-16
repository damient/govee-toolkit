// The SDK at startup, and what it knows before any scan.

import assert from "node:assert/strict";
import test from "node:test";

import { withGovee } from "./helpers.mjs";
import * as sdk from "../dist/index.js";

const { Config, Govee, MODES } = sdk;

test("start and close", async () => {
  const govee = await Govee.start(new Config());
  await govee.close();
});

test("no device is known before a scan", async () => {
  await withGovee(sdk, (govee) => {
    assert.deepEqual(govee.devices(), []);
  });
});

test("the modes are the transports this build carries", async () => {
  await withGovee(sdk, (govee) => {
    for (const mode of govee.modes()) {
      assert.ok(MODES.includes(mode), mode);
    }
  });
});

test("the default configuration raises no problem", async () => {
  await withGovee(sdk, (govee) => {
    assert.deepEqual(govee.problems(), []);
  });
});

test("the configuration in force is the one it started with", async () => {
  await withGovee(sdk, (govee) => {
    assert.deepEqual(govee.config.defaultModes, ["lan"]);
  });
});

test("the catalog is reachable from the facade", async () => {
  await withGovee(sdk, (govee) => {
    assert.ok(govee.catalog.skus().length > 0);
  });
});

test("the events of one SDK are iterated with for await", async () => {
  await withGovee(sdk, (govee) => {
    assert.equal(typeof govee.events()[Symbol.asyncIterator], "function");
  });
});
