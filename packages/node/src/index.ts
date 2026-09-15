// govee-toolkit — Node SDK entry point.
//
// A napi-rs binding over `packages/rust`, as `packages/python` is a PyO3 one.
// No protocol logic, no device file reading and no mode selection lives here:
// the core does all three, and this package hands its answers to JavaScript.
//
// TODO: public API, one module per shape the Python binding already exposes:
//   govee.ts      — start, scan, devices, modes, events
//   device.ts     — one handle: send, read, status, and the role verbs
//   catalog.ts    — the embedded device files
//   config.ts     — the configuration in force
//   stream.ts     — the raw segment channel
//   types.ts      — what the core reports, as the records it serializes

export {};
