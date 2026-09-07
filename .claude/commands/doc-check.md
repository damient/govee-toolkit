---
description: Check the current branch's docs and comments against the code, then clean up what drifted
allowed-tools: Bash, Read, Edit, Write, Glob, Grep
---

Check the documentation and the comments on the current branch against the code
they describe, then fix what is wrong.

Arguments: $ARGUMENTS — a path or a doc name narrows the check to it. With no
argument, check everything the branch touched.

## Scope

`git diff --stat main...HEAD` for the files the branch changed, and
`git diff main...HEAD` for what changed in them. Docs the branch did not touch
are in scope too when the branch changed the code they describe.

## Check

**1. Docs against code.** For every claim in `README.md`, `CLAUDE.md`,
`docs/**` and `tools/README.md` that the branch's code touches: is it still
true? Names, paths, feature flags, commands, exit codes, file layout, what is
implemented and what is not. Verify against the code — never assume the doc is
right because it reads well.

**2. Code with no doc.** Something the branch added that `CLAUDE.md`'s "Where
things go" table places in a doc, and which is not there: a new command, a new
tool script, a new cargo feature, a device file field, a protocol detail. Note
it; do not invent the missing content beyond what the code states.

**3. Comments and doc comments.** This is an audit, not a skim. Enumerate
every comment the branch added or changed — `//`, `///` and `//!` alike, and
the `#` comments in `devices/*.yaml`, `tools/*.sh` and the workflows — and rule
on each one. `git diff main...HEAD -U0 | grep -nE '^\+\s*(//|#)'` gets the
list; a comment is one unit with its continuation lines, not one line at a time.

A comment survives only by carrying something the code cannot state itself.
Name which one, per comment:

- a constraint the compiler does not enforce — an ordering, an invariant, a
  protocol rule, a reason a lock or an `await` sits exactly there;
- a measured value, with what it was measured on;
- a trap — what breaks if the obvious edit is made, why the slow or ugly form
  is the right one, why the tempting simplification is wrong;
- a pointer that saves a search — `docs/protocol/lan.md` §2.1, a device file
  field, an RFC;
- for `///` and `//!`: what a caller must know to use the item correctly —
  the errors it returns, the units, what it does not do.

Nothing else survives. Delete, do not reword:

- restatement — the comment says what the next line says in prose;
- a heading over code that names itself (`// Parse the reply`, `// Helpers`);
- narration of the obvious control flow (`// loop over devices`);
- a `///` on a **private** item that expands its name into a sentence
  (`/// Returns the cache path.` over `fn cache_path`);
- filler and framing — "simply", "just", "nice", "elegant", "robust",
  "powerful", "carefully", "of course";
- a comment that is true of the whole file repeated on each item;
- a commented-out line of code. If it is a real alternative, say so in one
  line; otherwise it goes.

**The `missing_docs` exception.** `packages/rust` sets `missing_docs = "warn"`
and `qa.sh` runs with `-D warnings`, so every public item must carry a `///`:
there, the rule is not deletion but payload. Make it say what the signature
cannot — the errors, the units, the ordering, what the item does not do. When
an item genuinely has nothing beyond its name, one short line is the lint's
price; never a paragraph, and never the same sentence the type above it
already carries.

Then compress what survives. A kept comment states its fact and stops: no
preamble, no restating the mechanism the reader is looking at, no second
sentence that repeats the first with different words. Two comments a few lines
apart making one point become one comment. A three-line comment that carries
one clause becomes one line.

The same applies to prose. In every doc the branch touched, cut the sentence
that repeats the previous one, the paragraph that re-explains a rule stated in
its own document, and the adjective that adds no fact. Two documents saying the
same thing means one of them owns it and the other links to it — `CLAUDE.md`,
`CONTRIBUTING.md` and `docs/` repeating each other is drift waiting to happen.
The four non-negotiables are the standing exception: `CLAUDE.md` and
`GOVERNANCE.md` both state them in full, on purpose. Leave both.

**Keep**, against all of the above: blunt phrasing where it is a real technical
caveat. Silent clamping, cloud-only features and explicit failures must stay
unambiguous, and length is not the problem there. English throughout.

**The target is half.** Count the comment lines the branch added or changed:

```bash
git diff main...HEAD -U0 | grep -cE '^\+\s*(//|#)'
```

The pass is not done until what stands in their place is **at most half** that
number. That is the calibration, not a quota to hit by deleting a fact: cut
under the rules above until the number falls, and where a file genuinely
cannot go further, say which file and why in the report. A branch that came in
near half already is finished at whatever it reaches — the rule catches the
usual case, which is prose two to three times longer than the fact it carries.

Where the cut is hard to find, it is almost always one of these:

- a doc comment whose second paragraph re-argues a rule `CLAUDE.md`,
  `docs/modes.md` or the module header already states. Delete the paragraph and
  keep the pointer;
- a `//!` header that narrates the whole design. It owes the reader the traps
  and the pointers, not a tour;
- a `///` on a field whose type and name already say it. Under `missing_docs`,
  cut it to the shortest line that satisfies the lint;
- a sentence built as "X, because otherwise Y would Z". Keep X, drop the
  counterfactual.

Report a count: comment lines added by the branch, lines after the pass, the
ratio, and how many comments were deleted outright rather than compressed.

**4. No negative or historical framing.** Docs and comments describe the code
as it is now. Refactored, renamed or deleted code leaves no trace in them.

- Cut "no longer", "used to", "previously", "was renamed to", "this replaces
  the old X", "moved out of Y", "since the split", "the former Z".
- Cut apology and deficit framing about the code itself — "unfortunately",
  "this is a hack until", "ideally this would" — unless it states a live
  constraint a reader must work around.
- Rewrite in the present: say what the thing is and what holds now.
- **Exception:** history that is load-bearing stays. A check or a test that
  exists to stop a specific regression may name it, a migration note a user
  needs to act on stays, and `CHANGELOG` entries, commit messages, `docs/
  roadmap.md` and release notes are history by design — leave them alone.

**5. ASD-STE100.** Every comment and every line of prose the branch touched
must follow the Simplified Technical English rules in `CLAUDE.md`, "Writing".
Rewrite what breaks one:

- a passive sentence becomes active;
- a sentence over 20 words (instruction) or 25 words (description) is split, or
  the half that carries no fact is cut;
- an `-ing` form that is not a name becomes a simple tense;
- two words for one object become one word, used everywhere;
- a dropped article or a dropped "that" comes back;
- a run of more than 3 nouns is broken with a preposition;
- an idiom, a metaphor or a rhetorical question is replaced with the fact it
  stood for;
- "should" becomes "must" for an obligation and "can" for a possibility;
- a sentence with more than one condition becomes a list.

Cut first, rewrite second: a sentence that carries no fact is deleted under
rule 3 and never rewritten into a shorter one.

**6. Device file `notes:`.** Every `notes:` the branch added or changed, on a
command, on a mode and in `verified:`. These are read by somebody deciding
whether to trust a command, so they carry evidence and nothing else:

- **what the firmware does**, in the terms the wire uses — which byte, which
  field, what the device did when it received it;
- **what was observed, and on what.** "Both were sent to the unit" is
  evidence. "Should work" is not, and neither is a range read off the vendor's
  app;
- **the pointer**: `docs/protocol/<mode>.md` §N, required on `documented:
  false`;
- **a trap**: silent clamping, a field that must accompany another, a mask the
  firmware drops without saying so;
- **an open question**, as an explicit `TODO`.

Delete from a `notes:`, do not reword: a restatement of the `frame:` above it
token by token, a restatement of the argument's own `range:`, a sentence
arguing why the SDK behaves as it does — that belongs in `docs/` and in the
Rust — and any claim nobody established. `notes:` is not the place to teach
the schema: the schema is `devices/schema.yaml`.

The `verified:` block is the one place where length is a virtue, since it
records exactly what somebody exercised and what they did not. Cut nothing
from it that names an observation; cut everything from it that does not.

## Fix

Apply the fixes. Edit prose and comments freely; do not change code behavior to
make a doc true — if a doc and the code disagree on what the code should do,
stop and ask which one is right.

After editing any device file, run `cargo run -p xtask -- compat`. Never fill a
`verified`, a capability or a measurement from inference — an unverified fact
is `?` or a `TODO`.

## Report

A table: file, what was wrong (drift / redundant comment / negative framing /
missing doc), and the fix. Then list separately anything you did not fix and
why. Do not commit or push.
