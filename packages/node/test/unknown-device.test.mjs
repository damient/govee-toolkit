// A handle for an identity no scan found. The send path refuses to scan, so
// such a device fails with `unknown_device` before a command is encoded.

import assert from "node:assert/strict";
import test from "node:test";

import { refusesUnknown, UNKNOWN_ID, withGovee } from "./helpers.mjs";
import * as sdk from "../dist/index.js";

const { MODES } = sdk;

/** Run one test on a handle for that identity, over `lan`. */
function onHandle(run) {
  return withGovee(sdk, async (govee) => {
    if (!govee.modes().includes("lan")) {
      return;
    }
    await run(govee.device(UNKNOWN_ID), govee);
  });
}

test("a handle carries the identity and its modes", () =>
  onHandle((handle) => {
    assert.equal(handle.id, UNKNOWN_ID);
    assert.deepEqual(handle.modes, ["lan"]);
  }));

test("an unknown device has no health", () =>
  onHandle((handle) => {
    assert.equal(handle.health("lan"), null);
  }));

test("send refuses an unknown device", () =>
  onHandle((handle) => refusesUnknown(() => handle.send("power"))));

test("a verb refuses an unknown device", () =>
  onHandle((handle) => refusesUnknown(() => handle.power(true))));

test("read refuses an unknown device", () =>
  onHandle((handle) => refusesUnknown(() => handle.read("status"))));

test("status refuses an unknown device", () =>
  onHandle((handle) => refusesUnknown(() => handle.status())));

test("the specification needs a known device", () =>
  onHandle((handle) => refusesUnknown(() => handle.spec())));

test("the description needs a known device", () =>
  onHandle((handle) => refusesUnknown(() => handle.describe())));

test("the serving mode needs a known device", () =>
  onHandle((handle) => refusesUnknown(() => handle.servingMode())));

test("no mode watches a device no mode knows", () =>
  onHandle((handle) => {
    assert.equal(handle.watchStatus(), null);
  }));

test("every mode the package names reads back", () =>
  onHandle((handle) => {
    for (const mode of MODES) {
      assert.equal(handle.health(mode), null);
    }
  }));

test("a name that is no mode says which ones are", () =>
  onHandle((handle) => {
    assert.throws(() => handle.health("wifi"), (error) => {
      assert.match(error.message, /lan, ble, cloud/);
      return true;
    });
  }));
