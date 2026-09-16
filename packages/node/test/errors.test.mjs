// What a failure carries: the code the core gives it, and the family.

import assert from "node:assert/strict";
import test from "node:test";

import "./helpers.mjs";
import { Catalog } from "../dist/index.js";

function raised(call) {
  try {
    call();
  } catch (error) {
    return error;
  }
  return assert.fail("the call was expected to fail");
}

test("a failure is a JavaScript error", () => {
  assert.ok(raised(() => Catalog.embedded().device("H0000")) instanceof Error);
});

test("a failure carries the code the core gives it", () => {
  assert.equal(raised(() => Catalog.embedded().device("H0000")).code, "unknown_sku");
});

test("a failure names the family it belongs to", () => {
  assert.equal(raised(() => Catalog.embedded().device("H0000")).name, "CodecError");
});

test("the message says what was asked for", () => {
  assert.match(raised(() => Catalog.embedded().device("H0000")).message, /H0000/);
});
