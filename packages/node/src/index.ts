// govee-toolkit — Node SDK entry point.
//
// A napi-rs binding over `packages/rust`, as `packages/python` is a PyO3 one.
// No protocol logic, no device file reading and no mode selection lives here:
// the core does all three, and this package hands its answers to JavaScript.
//
// TODO: the public API, one module per shape `packages/python/src` exposes.

export {};
