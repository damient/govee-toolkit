# What changed

<!-- One or two sentences. Link the issue if there is one. -->

## Modes

<!-- Which of `lan`, `ble`, `cloud` this touches, or none. -->

- [ ] It changes the `lan` send path. If so, say what it adds and why the cost
      is justified — latency and reliability there come before anything else.

## Checks

- [ ] `tools/qa.sh` run locally and green.
- [ ] A conformance vector under `tests/fixtures/golden/` for every new command,
      with its `source` saying whether the bytes come from a capture or were
      worked out from the documented layout.
- [ ] A changelog entry for what changed — `packages/<pkg>/CHANGELOG.md` for a
      package's behavior, the root `CHANGELOG.md` for the catalogue, the docs,
      the tooling and CI.
- [ ] Every commit subject is `<type>(<scope>)!: <summary>`, and every commit
      carries `Signed-off-by:` (`git commit -s`).
- [ ] Anything not verified on hardware is left as `?` or `TODO`, not filled in
      from inference.

## Provenance

- [ ] No artefact taken out of Govee software — no decompiler or disassembler
      output, extracted strings, resources or firmware image.
- [ ] No unredacted capture: no MAC address, IP address, API key or account
      token. A pull request carrying one is closed rather than amended, because
      the history keeps what a later commit removes.
