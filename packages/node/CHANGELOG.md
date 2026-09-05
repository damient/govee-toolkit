# Changelog

Changes to `govee-toolkit` (Node.js), the binding over the Rust core in
[`../rust`](../rust). The policy is
[`../../docs/versioning.md`](../../docs/versioning.md).

### Added

- Package scaffolding: `package.json` declaring the name `govee-toolkit` (ESM,
  no dependencies), `tsconfig.json`, and `src/index.ts`.
- `typescript` as the only devDependency, with `package-lock.json` committed, so
  `npm ci && npm run build` runs from a clean checkout.
- Package metadata: `author`, `repository` (with `directory`), `homepage`,
  `bugs`, `keywords` and `engines`.

### Changed

- `npm test` scans the package for test files instead of being handed `test/`,
  which the runner read as a missing module while the directory was empty.
- The package README links to `docs/` and `devices/` by absolute URL: it is the
  description shown on npm, where a link out of the package directory is dead.

### Removed

- The non-standard `comment` key from `package.json`, which npm would have
  published as-is.
