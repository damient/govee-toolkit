// How a JavaScript value crosses into a command argument.
//
// The binding reads the shape of a value and the device file states its type:
// the send path reads an array of whole numbers as zone indices, byte values
// or one color, under what the entry declares. A shape the binding takes
// reaches that path and fails there, on the unknown device these tests use; a
// shape it takes for no argument is a `TypeError` and nothing is sent either
// way.

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

for (const resolution of ["app", "native", 12]) {
  test(`a resolution is a name or a zone count: ${resolution}`, () =>
    onHandle((handle) => refusesUnknown(() => handle.openStream(resolution))));
}

test("an unnamed resolution is refused", () =>
  onHandle((handle) => refusesValue(() => handle.openStream("every"))));

test("an unnamed rate is refused", () =>
  onHandle((handle) => refusesValue(() => handle.openStream(null, "fast"))));

test("an unnamed mode is refused", () =>
  withGovee(sdk, (govee) => refusesValue(() => govee.scanOn(["radio"]))));
