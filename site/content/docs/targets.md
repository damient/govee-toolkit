---
title: Names and groups
slug: targets
order: 4
description: Give a device a name, put several devices in a group, and drive them by that name instead of a MAC address.
---

# Names and groups

A MAC address is hard to type and harder to remember. Give each device a name
in `config.yaml`, put devices that go together in a group, and every command
takes those names in place of the MAC address.

## Name a device

`name:` gives one device a name. `groups:` puts it in one group or more:

```yaml
devices:
  "AA:BB:CC:DD:EE:FF":
    name: "kitchen"
    groups: [ambient]
  "11:22:33:44:55:66":
    name: "desk"
    groups: [ambient, office]
  "22:33:44:55:66:77":
    name: "shelf"
    groups: [office]
```

A group exists as soon as one device carries it. There is no list of groups to
keep in step. A name and a group match whole and ignore case, so `Kitchen`
finds `kitchen` and `kit` finds nothing.

## Drive a group

A command sent to a group goes to every member at once:

<div class="terminal">
<pre><code><span class="prompt">$</span> govee on ambient
<span class="prompt">$</span> govee brightness ambient 40</code></pre>
<button class="copy" type="button" data-copy="govee brightness ambient 40">Copy</button>
</div>

Each member answers on its own:

- A member that fails stops no other one. Its line goes to stderr, and the
  command exits with the code of the first member that failed.
- Each member uses its own modes. `--mode` restricts every member to that
  mode. A member that does not enable it fails, and the others go ahead.
- Each member reads the value against its own range. One brightness can be in
  range for one member and out of range for another.

## Write a target

A target is what you type where a command expects a device. It takes one of
four forms:

| Target | It names |
| ------ | -------- |
| `AA:BB:CC:DD:EE:FF` | the device with that identity |
| `kitchen` | the device the configuration names `kitchen` |
| `ambient` | every device in the group `ambient` |
| `H6159` | every known device of that model |

A command that controls a light, such as `on` or `brightness`, takes an
identity, a name or a group. `govee devices` and `govee identify` take any
number of targets, a model included.

A command that controls a light reads the name or the group from
`config.yaml` alone, with no scan. `govee devices` lists what is already
known, so scan first: a model, a name and a group match among the devices a
scan found.
