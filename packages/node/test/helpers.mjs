// What every test in this directory shares.
//
// No test here reaches hardware and no test here reaches the network. The
// catalog is compiled into the addon, and a device that no scan found fails
// before any byte leaves the process.

import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

// Point every path the core reads at an empty temporary directory. The core
// reads the configuration file, the device cache and a `.env` from the
// machine. A cache written by a real run puts devices in `govee.devices()`
// that no test discovered, and the checkout's own `.env` names a
// configuration file through `GOVEE_CONFIG`. `GOVEE_ENV_FILE` replaces the
// `.env` search, and the process environment wins over a file, so both are
// set here.
const state = mkdtempSync(join(tmpdir(), "govee-node-test-"));
const empty = join(state, "env");
writeFileSync(empty, "");
process.env.XDG_CONFIG_HOME = join(state, "config");
process.env.XDG_CACHE_HOME = join(state, "cache");
process.env.GOVEE_ENV_FILE = empty;
// A file that is not there is the default configuration, not an error.
process.env.GOVEE_CONFIG = join(state, "config.yaml");

export const here = dirname(fileURLToPath(import.meta.url));
export const packageRoot = resolve(here, "..");
export const repositoryRoot = resolve(packageRoot, "..", "..");

/** A release number, as every manifest in this repository writes one. */
export const VERSION_PATTERN = /^\d+\.\d+\.\d+/;

/** An identity no scan found. Every command for it fails, and says so. */
export const UNKNOWN_ID = "AA:BB:CC:11:22:33";

/** Run one test against a started SDK, and close it afterwards. */
export async function withGovee(sdk, run) {
  const { Govee, Config } = sdk;
  const govee = await Govee.start(new Config());
  try {
    await run(govee);
  } finally {
    await govee.close();
  }
}

/**
 * Assert the call fails with `unknown_device`, and nothing was sent.
 *
 * The send path refuses to scan, so every call on an identity no transport
 * knows ends the same way. A call that reaches it is a call the binding
 * accepted the arguments of.
 */
export async function refusesUnknown(call) {
  await assert.rejects(async () => call(), (error) => {
    assert.equal(error.code, "unknown_device");
    assert.equal(error.name, "TransportError");
    return true;
  });
}

/** Assert the binding refused the value before the core saw it. */
export async function refusesValue(call) {
  await assert.rejects(async () => call(), (error) => {
    assert.ok(error instanceof TypeError, `${error.name} is not a TypeError`);
    assert.equal(error.code, "invalid_argument");
    return true;
  });
}
