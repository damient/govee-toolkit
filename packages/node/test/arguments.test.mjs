// How a JavaScript value crosses into a command argument.
//
// The binding reads the shape, and the device file states the type. A shape
// the binding takes reaches the send path and fails there, on the unknown
// device these tests use. A shape it takes for no argument is a `TypeError`.

import assert from "node:assert/strict";
import test from "node:test";

import { refusesUnknown, refusesValue, UNKNOWN_ID, withGovee } from "./helpers.mjs";
import * as sdk from "../dist/index.js";

function onHandle(run) {
  return withGovee(sdk, async (govee) => {
    if (!govee.modes().includes("lan")) {
      return;
    }
    await run(govee.device(UNKNOWN_ID), govee);
  });
}

const ACCEPTED = {
  boolean: true,
  number: 7,
  string: "text",
  bytes: new Uint8Array([1, 2]),
  buffer: Buffer.from([1, 2]),
  "a typed array of another element type": new Int16Array([1, 2]),
  colors: [[255, 0, 0], [0, 255, 0]],
  "whole numbers": [0, 1, 2],
};

const REFUSED = {
  object: { a: 1 },
  null: null,
  "a number with a fraction": 1.5,
};

for (const [shape, value] of Object.entries(ACCEPTED)) {
  test(`an accepted value reaches the send path: ${shape}`, () =>
    onHandle((handle) => refusesUnknown(() => handle.send("power", { value }))));
}

for (const [shape, value] of Object.entries(REFUSED)) {
  test(`a shape no argument takes is refused: ${shape}`, () =>
    onHandle((handle) => refusesValue(() => handle.send("power", { value }))));
}

test("a color takes three channels", () =>
  onHandle((handle) => refusesValue(() => handle.color([255, 0]))));

test("a channel outside 0 to 255 is refused", () =>
  onHandle((handle) => refusesValue(() => handle.color([256, 0, 0]))));

test("a color reaches the send path", () =>
  onHandle((handle) => refusesUnknown(() => handle.color([255, 0, 0]))));

// Both forms state the same color. A channel outside 0 to 255 has no test
// here: `Uint8Array` wraps the value before the binding reads it.

test("a color is three bytes", () =>
  onHandle((handle) => refusesUnknown(() => handle.color(new Uint8Array([255, 0, 0])))));

test("a color of two bytes is refused", () =>
  onHandle((handle) => refusesValue(() => handle.color(new Uint8Array([255, 0])))));

test("a paint is three bytes for every zone", () =>
  onHandle((handle) =>
    refusesUnknown(() => handle.segment(new Uint8Array([255, 0, 0, 0, 255, 0])))));

test("a paint that is no whole number of colors is refused", () =>
  onHandle((handle) => refusesValue(() => handle.segment(new Uint8Array([255, 0, 0, 0])))));

for (const resolution of ["app", "native", "groups", 12]) {
  test(`a resolution is a name or a zone count: ${resolution}`, () =>
    onHandle((handle) => refusesUnknown(() => handle.openStream(resolution))));
}

test("an unnamed resolution is refused", () =>
  onHandle((handle) => refusesValue(() => handle.openStream("every"))));

test("an unnamed rate is refused", () =>
  onHandle((handle) => refusesValue(() => handle.openStream(null, "fast"))));

test("an unnamed mode is refused", () =>
  withGovee(sdk, (govee) => refusesValue(() => govee.scan(["radio"]))));

test("a drawing carries colors", () =>
  onHandle((handle) => refusesValue(() => handle.draw({ cells: [0, 1] }))));

test("a drawing key outside the list is refused", () =>
  onHandle((handle) => refusesValue(() => handle.draw({ colors: [255, 0, 0], zones: [0] }))));

test("a drawing argument of whole numbers reaches the send path", () =>
  onHandle((handle) =>
    refusesUnknown(() => handle.draw({ colors: [255, 0, 0], args: { background: [1, 2, 3] } }))));

test("an unnamed drawing resolution is refused", () =>
  onHandle((handle) => refusesValue(() => handle.draw({ colors: [255, 0, 0], resolution: "every" }))));

test("a drawing reaches the send path", () =>
  onHandle((handle) =>
    refusesUnknown(() =>
      handle.draw({ cells: [0, 5], colors: [[255, 0, 0]], args: { speed: 50 } }))));

const SCENE = { name: "Aurora A", category: "Natural", code: 2867, sceneType: 1, param: new Uint8Array([1, 2]) };

for (const scene of ["Aurora", 2867, SCENE]) {
  test(`a scene is a name, a code or a Scene: ${typeof scene}`, () =>
    onHandle((handle) => refusesUnknown(() => handle.scene(scene))));
}

const REFUSED_SCENES = {
  boolean: true,
  "a Scene with no param": { ...SCENE, param: undefined },
  "a code outside 0 to 65535": { ...SCENE, code: 70000 },
  "a key outside the Scene": { ...SCENE, effect: 1 },
};

for (const [shape, scene] of Object.entries(REFUSED_SCENES)) {
  test(`a scene shape is refused: ${shape}`, () =>
    onHandle((handle) => refusesValue(() => handle.scene(scene))));
}

test("a set takes a scene name or code alone", () =>
  onHandle(async (_, govee) => {
    const devices = await govee.devices([UNKNOWN_ID]);
    await refusesValue(() => devices.scene(SCENE));
  }));

test("a library that no fetch stored is missing", () =>
  assert.throws(() => sdk.loadScenes("H6022"), { code: "scene_list_missing" }));

test("a SKU that no file name carries is refused", () =>
  assert.throws(() => sdk.loadScenes("../H6022"), { code: "bad_sku" }));
