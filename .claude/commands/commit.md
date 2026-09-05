---
description: Commit the working tree as a few coherent commits, in the repository's conventions
allowed-tools: Bash, Read, Glob, Grep
---

Commit what is in the working tree, grouped into as few coherent commits as the
work allows.

Arguments: $ARGUMENTS — a path or a subject narrows the commit to it. With no
argument, everything in the tree is committed, untracked files included.

## Screen first

Nothing is committed before the tree has been looked at. `git status --short`
and `git diff --stat`, then read anything you did not write yourself.

Never commit, and never stage:

- A secret or a credential: `.env` and `.env.*`, `*.pem`, `*.key`, `*.p12`,
  `id_rsa*`, `credentials*`, `*token*`, `*secret*`, a `config.yaml` carrying an
  API key.
- Local editor and agent state: `.claude/settings.local.json`, `.vscode/`,
  `.idea/`, `*.local`.
- Build output and caches: `dist/`, `target/`, `node_modules/`, `out/`,
  `__pycache__/`, `.cache/`, `*.tsbuildinfo`. `dist/catalog.json` is generated
  by `xtask` and is never committed.
- Anything taken out of Govee software: decompiler or disassembler output, an
  extracted string table, a resource, a firmware image. `CONTRIBUTING.md`,
  "Legal and provenance", is the rule and it has no exception.
- An unredacted capture. A file under `tests/fixtures/*-captures/` carries a
  real MAC, IP, SSID or token until somebody redacted it, and git keeps it
  after the fix.
- A file over a megabyte, or one whose diff you cannot read, until the user has
  said what it is.

Report what you left out and why, by name. Do not silently drop a file, and do
not commit one you are unsure about — ask.

`git add` takes explicit paths. Never `git add -A`, `git add .` or `git commit
-a`: they stage what the screen above just excluded.

## Group

One commit is one subject a reader can act on, with everything that subject
needs: the code, its tests, the doc that describes it, and its changelog entry.
A commit per file is noise, and one commit for two unrelated subjects hides
both.

- Group by what changed, not by file type. A new capability and the docs and
  vectors that come with it are one commit; two unrelated fixes are two.
- The type is the dominant one for that subject, and the scope is the area it
  touches — `feat(lan)`, `fix(codec)`, `docs`, `ci`, `chore`.
- Two commits on the same subject mean one commit. Two subjects sharing a type
  are still two commits.
- Order them so each one is a tree that stands on its own: a rename before what
  uses the new name, a device file before the vector that replays it.
- Above four or five commits for one working tree, stop and say what the groups
  are before committing — the tree probably holds more than one branch's worth
  of work.

## Write the message

```
<type>(<scope>)!: <summary>

<body>

Signed-off-by: <author>
```

- `type` is one of `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `build`,
  `ci`, `chore`, `revert`. `scope` is optional and lowercase — `lan`, `codec`,
  `stream`, `devices`, `node`, `python`, `ci`. A `!`, or a `BREAKING CHANGE:`
  trailer, marks a change that breaks the public API. The type decides the
  semver bump at release, so `feat` and `fix` are not interchangeable.
- The summary is imperative and under 72 characters, with no trailing period.
- The body is where the reasoning goes, wrapped at 72: what the change does,
  and why it is shaped that way. What the diff already says needs no
  paragraph. Plain and concise, English, no selling — `CLAUDE.md`, "Writing".
- A commit message is history by design, so it may say what a change replaces
  and why the old shape did not hold. That licence stops at the message: the
  docs and comments it ships still describe the code as it is.
- `git commit -s` for the sign-off. CI rejects a commit without it, and the
  line has to match the author.

## Guards

Before each commit, and only when that commit touches the area:

- `tools/check-captures.sh` when it stages anything under `tests/fixtures/` or
  `devices/`. It reads tracked files, so run it after `git add` and before
  `git commit`.
- `cargo run -p xtask -- compat` from `packages/rust` when a device file
  changed, and stage the regenerated `docs/compatibility.md` with it. CI fails
  on drift.
- A changelog entry when it stages `packages/*/src/`, a `build.rs` or a device
  file — `/changelog` writes it. CI fails without one.

`tools/qa.sh` is not part of committing. Run `/qa` before pushing.

## Finish

`git log --oneline` for what you made, and `git status --short` to show what is
left. Report each commit as one line, then the files you excluded and why.

Do not push, do not tag, and do not open or update a pull request.
