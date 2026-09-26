// The SDK at startup, and what it knows before any scan.

import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { refusesValue, withGovee } from "./helpers.mjs";
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

test("an identity selects itself, and a model nothing found selects nothing", async () => {
  await withGovee(sdk, (govee) => {
    assert.deepEqual(govee.select(["aa:bb:cc:dd:ee:ff:00:11"]), [
      "AA:BB:CC:DD:EE:FF:00:11",
    ]);
    assert.throws(() => govee.select(["H6008"]), { code: "no_such_target" });
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

const NAMED = `devices:
  "AA:BB:CC:DD:EE:FF":
    name: kitchen
  "11:22:33:44:55:66":
    name: twin
  "22:33:44:55:66:77":
    name: twin
`;

test("a handle takes a name the configuration gives", async () => {
  const path = join(mkdtempSync(join(tmpdir(), "govee-")), "config.yaml");
  writeFileSync(path, NAMED);
  const govee = await Govee.start(Config.loadFrom(path));
  try {
    assert.equal(govee.device("kitchen").id, "AA:BB:CC:DD:EE:FF");
    assert.equal(govee.deviceOn("name:KITCHEN", "lan").id, "AA:BB:CC:DD:EE:FF");
    assert.equal(govee.device("id:AA:BB:CC:DD:EE:FF").id, "AA:BB:CC:DD:EE:FF");
    for (const [target, code] of [
      ["name:attic", "no_such_target"],
      ["twin", "ambiguous_target"],
      ["sku:H6008", "target_not_understood"],
    ]) {
      assert.throws(() => govee.device(target), { code }, target);
    }
  } finally {
    await govee.close();
  }
});

const GROUPED = `devices:
  "BB:00:00:00:00:02":
    name: hall
    groups: [ambient]
  "AA:00:00:00:00:01":
    groups: [Ambient]
    modes: [ble]
`;

test("a group names its members, and each member answers alone", async () => {
  const path = join(mkdtempSync(join(tmpdir(), "govee-")), "config.yaml");
  writeFileSync(path, GROUPED);
  const govee = await Govee.start(Config.loadFrom(path));
  try {
    const members = ["AA:00:00:00:00:01", "BB:00:00:00:00:02"];
    assert.deepEqual(govee.targets("ambient"), members);
    assert.deepEqual(govee.targets("hall"), ["BB:00:00:00:00:02"]);
    const group = govee.group("group:ambient", "lan");
    assert.deepEqual(group.members, members);

    const outcomes = await group.power(true);
    assert.deepEqual(
      outcomes.map((outcome) => outcome.id),
      members,
    );
    assert.ok(outcomes.every((outcome) => !outcome.ok));
    assert.equal(outcomes[0].error.code, "mode_not_enabled");
    assert.ok(outcomes[1].error instanceof Error);
    assert.equal(outcomes[1].error.code, "unknown_device");
    assert.equal(outcomes[1].served, null);

    assert.throws(() => govee.device("ambient"), { code: "target_not_understood" });
  } finally {
    await govee.close();
  }
});

test("an identify walk over an empty list walks no device", async () => {
  await withGovee(sdk, async (govee) => {
    const report = await govee.identify([]);
    assert.deepEqual(report.lit, []);
    assert.deepEqual(report.failed, []);
    assert.deepEqual(report.stayed, []);
    assert.equal(report.ok, true);
  });
});

test("an identify option the list does not name is refused", async () => {
  await withGovee(sdk, async (govee) => {
    await refusesValue(() => govee.identify([], { holdMS: 0 }));
    await refusesValue(() => govee.identify([], { holdMs: -1 }));
    await refusesValue(() => govee.identify([], { waitMs: 0.5 }));
    await refusesValue(() => govee.identify([], { color: [256, 0, 0] }));
    await refusesValue(() => govee.identify([], { mode: "radio" }));
  });
});

test("an identify walk over a mode the device does not enable is refused before a scan", async () => {
  const path = join(mkdtempSync(join(tmpdir(), "govee-")), "config.yaml");
  writeFileSync(path, GROUPED);
  const govee = await Govee.start(Config.loadFrom(path));
  try {
    await assert.rejects(
      () => govee.identify("AA:00:00:00:00:01", { waitMs: 0, holdMs: 0 }),
      { code: "mode_not_enabled" },
    );
  } finally {
    await govee.close();
  }
});
