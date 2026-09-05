---
description: Write the changelog entries for the current branch, in the repository's format
allowed-tools: Bash, Read, Edit, Write, Glob, Grep
---

Write the changelog entries for what the current branch changed.

Arguments: $ARGUMENTS — a package name (`rust`, `python`, `node`) narrows the
work to that file. `--pr` also brings the pull request title and body in step
with the same set of changes.

## Where an entry goes

| Change | File |
| ------ | ---- |
| A package's public API, features, behavior | `packages/<pkg>/CHANGELOG.md` |
| `devices/*.yaml`, `devices/schema.yaml`, the conformance vectors | `CHANGELOG.md` |

The catalogue is shared, so its changelog is shared: one entry serves the three
packages that embed it, rather than the same device fix described three times.
The root file also keeps the index of the packages and the version each manifest
carries.

A change to the docs, `tools/`, the CI or the repository layout gets no entry.
It ships to nobody, its rules live in `CONTRIBUTING.md` and `docs/`, and its
history is the git log.

CI draws the same line: a pull request touching `packages/*/src/`, a `build.rs`
or a `devices/*.yaml` fails without a changed `CHANGELOG.md`, and the
`no-changelog` label is the exception. A branch that touches none of them owes
nothing — say so and stop rather than inventing an entry.

A change that touches both a package and the catalogue gets an entry in each,
written from that file's point of view — the package entry names the API, the
root entry names the data. Never the same sentence twice.

## Format

Keep a Changelog structure, as the large public repositories use it. The header
of each file states what it covers and stops; the format needs no introduction
of its own, and no link to a specification.

```markdown
## [0.3.0] — 2026-04-12

### Added

- `Govee::subscribe()` — the event stream, one `Event` per mode transition.

### Changed

- **Breaking:** `send()` takes `&Args`. Pass a reference at every call site.
```

- In a package file, one `##` heading per version: `## [X.Y.Z] — YYYY-MM-DD`,
  newest first. A version the manifest already declares carries its number with
  no date, and gains the date on the day it is tagged.
- In the root file, the catalogue's headings are dates — `### 2026-09-05`,
  newest first, sections at `####`. The catalogue has no version of its own: a
  package embeds it at build time, so the date is what a release pins.
- In a package file, entries for work whose version is not settled sit under
  their `###` sections at the top, above the newest `##` heading. Cutting the
  version puts the heading in above them.
- In the root file, entries go under the date heading at the top when it is
  today's, and under a new one when it is older. Take the date from `date +%F`,
  never from memory.
- There is no `Unreleased` heading: it names a state rather than a change, and
  `tools/release-notes.sh` needs the number to find the section anyway.
- Sections, in this order, and only the ones with entries: `Added`, `Changed`,
  `Deprecated`, `Removed`, `Fixed`, `Security`. The commit type says which one:
  `feat` is `Added` — `Changed` when it reshapes something that existed — `fix`
  is `Fixed`, and `perf` and `refactor` are `Changed` when a caller can tell,
  nothing otherwise. `docs`, `test`, `build`, `ci` and `chore` earn no entry,
  unless one of them changed a device file: what the commit touched decides,
  not the type it declared.
- A bullet is one change, stated as a fact in the present tense. Lead with the
  public name — the item, the command, the file, the SKU — then what it does.
- A breaking change opens with `**Breaking:**` and says what to do instead.
- An entry links to the doc that carries the detail rather than reproducing it.
- Wrap at 80 columns. English, plain, no selling.

## What an entry says

Write what exists and what it does. The reader is looking for what changed for
them, and everything on the page is something they can use.

- State behavior, not absence. "Enabling `ble` reports the mode as unavailable"
  — not "`ble` is not implemented".
- No status prose about what has not happened: nothing shipped, no
  implementation yet, nobody using it, not part of this release. A version with
  no entries has no heading; that is the whole statement.
- No apology, no hedging, no framing against a past shape of the code.
- Keep a caveat that is a real technical fact the reader must act on: a silent
  clamp, a value that is measured per unit, an error where a caller expects a
  value. Blunt is fine; state it as behavior.

## Gather

Everything the entries are written from, in one pass:

```bash
git log main..HEAD --format='%h %s'   # types and scopes: section, and bump
git log main..HEAD --format='%B'      # bodies: what an entry is written from
git diff --name-only main...HEAD      # which committed files owe an entry
git status --short                    # and which new ones, not yet added
date +%F                              # the date a root heading carries
gh pr view --json number,title,body   # with --pr
```

An untracked file is part of the branch's work as much as a committed one: a
device file written this session owes its entry before it is added, not after.

A commit that does not parse as `<type>(<scope>)!: <summary>` predates the
convention or slipped past CI. Read it and place it by what it changed.

## Writing the entries

1. Group the commits by what a reader of that file would call one change. Two
   commits that fix and then refine the same thing are one entry; one commit
   that adds a capability and a device file is one entry per file.
2. Read the entries already at the top of the file — most branches extend an
   existing list rather than starting one. Merge into it; do not append a second
   bullet on a subject already covered.
3. Verify every name you write against the code: the item path, the feature
   flag, the CLI invocation, the config key. A changelog naming an item that
   does not exist is worse than no entry.
4. A device file change names the SKU, what changed, and whether the new
   behavior comes from a capture or from the documented layout. Never state a
   capability or a measurement the file does not carry.
5. An MSRV raise, a new cargo feature, a changed argument range and a renamed
   command each get their own entry — see `docs/versioning.md` for which bump
   they force.
6. `tools/release-notes.sh <pkg> <pkg>-vX.Y.Z` prints the section as the release
   will carry it. Run it when the branch bumps a version: it is what the release
   workflow runs, and it fails on a heading with no entries under it.
7. A branch that cuts a version also updates the `Version` column of the index
   table in the root file. The release history itself is the repository's
   releases page, built by the release workflow; no table in the repository
   repeats it.

## With `--pr`

`gh pr view --json number,title,body` gives the number and what is there now.
Write the new body to a file and edit the pull request with it:

```bash
gh pr edit "$number" --title '…' --body-file /tmp/pr-body.md
```

The body describes the branch's result, not its route: what the code does now,
in the same plain register as the changelog, with the same rule against
describing what is absent. Drop anything the branch has since removed or
renamed. Prose paragraphs, one per subsystem, with a short list at the end for
what does not need a paragraph.

## Finish

Report the files touched and the entries added, as a list. Then the bump the
commit types imply, so the version is a reading rather than a guess: a `!` or a
`BREAKING CHANGE:` trailer is the breaking bump — pre-1.0, `0.x` to `0.(x+1)` —
a `feat` is the minor one, a `fix` or a `perf` the patch. Name the highest one
in the range and the commit that forces it.

Do not bump a manifest, tag, commit or push.
