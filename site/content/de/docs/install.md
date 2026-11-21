---
title: Installation
slug: install
order: 2
description: Das Rust-Crate, die Kommandozeile, das Python-Paket, das Node.js-Paket und die DMX-Bridge.
source: d2d3c23cd5e0fcc83f9d4544b1089bd06b6d3958
---

# Installation

Das Rust-Crate ist die Referenzimplementierung. Die Kommandozeile funktioniert
direkt aus einem Terminal. Python und Node.js binden denselben Kern an. Die
DMX-Bridge ist ein eigenes Binary.

## Rust {{version_rust}} {{registry_rust}} {#rust}

Asynchron, auf Tokio. Das Feature `lan` ist standardmäßig aktiv. `ble` und
`cloud` schaltest du bei Bedarf dazu.

<div class="terminal">
<pre><code><span class="prompt">$</span> cargo add govee-toolkit
<span class="prompt">$</span> cargo add govee-toolkit --features ble,cloud</code></pre>
<button class="copy" type="button" data-copy="cargo add govee-toolkit">Kopieren</button>
</div>

```rust
use govee_toolkit::{Config, Filter, Govee};

let govee = Govee::start(Config::load()?).await?;
let devices = govee.devices(Filter::all(), None).await?;
devices.power(true).await;
```

Unter Linux erreicht das Feature `ble` den Funkchip über BlueZ und D-Bus. Der
Build braucht deshalb die dbus-Header (`apt install libdbus-1-dev`). macOS und
Windows bauen ohne Zusatz.

## Kommandozeile {{version_cli}} {{registry_cli}} {#command-line}

Das Binary heißt `govee`. Es liest die Gerätedateien über das Crate.

<div class="terminal">
<pre><code><span class="prompt">$</span> cargo install govee-toolkit-cli
<span class="prompt">$</span> govee scan
<span class="prompt">$</span> govee on DEVICE
<span class="prompt">$</span> govee color DEVICE "#ff3d00"</code></pre>
<button class="copy" type="button" data-copy="cargo install govee-toolkit-cli">Kopieren</button>
</div>

[Jeder Befehl, mit einem Beispiel in jeder Sprache]({{base}}docs/reference/)

## Python {{version_python}} {{registry_python}} {#python}

Ein PyO3-Binding über dem Rust-Kern. Die API ist ausschließlich `asyncio`, und
das Wheel enthält den Gerätekatalog. `pip install` ist also die ganze
Installation.

<div class="terminal">
<pre><code><span class="prompt">$</span> pip install govee-toolkit</code></pre>
<button class="copy" type="button" data-copy="pip install govee-toolkit">Kopieren</button>
</div>

```python
from govee_toolkit import Govee

govee = await Govee.start()
devices = await govee.devices()
await devices.power(True)
await govee.close()
```

Die Wheels sind `abi3` für Python 3.11 und neuer: Linux, macOS und Windows auf
`x86_64` und `aarch64`, Linux auf `armv7`, und musl auf `x86_64`, `aarch64` und
`armv7`. Auf jeder anderen Plattform baut pip aus dem Repository. Dafür braucht
es eine Rust-Toolchain.

## Node.js {{version_node}} {{registry_node}} {#node-js}

Ein napi-rs-Binding über demselben Kern, mit TypeScript-Typen. Das Paket enthält
den Gerätekatalog. `npm install` ist also die ganze Installation.

<div class="terminal">
<pre><code><span class="prompt">$</span> npm install govee-toolkit</code></pre>
<button class="copy" type="button" data-copy="npm install govee-toolkit">Kopieren</button>
</div>

```javascript
import { Govee } from "govee-toolkit"

const govee = await Govee.start()
const devices = await govee.devices()
await devices.power(true)
await govee.close()
```

Das Paket braucht Node.js 20 oder neuer. Das Addon gibt es für Linux und Windows
auf `x86_64` und `aarch64`, und für macOS auf `aarch64`. Eine andere Plattform
baut aus dem Repository, weil das Addon den Gerätekatalog aus
`devices/*.yaml` kompiliert.

## DMX-Bridge {{version_dmx}} {{registry_dmx}} {#dmx-bridge}

Das Binary heißt `govee-dmx`. Es empfängt Art-Net im Netzwerk und schreibt jeden
Frame über `lan` an eine Leuchte.

<div class="terminal">
<pre><code><span class="prompt">$</span> cargo install govee-toolkit-dmx
<span class="prompt">$</span> govee-dmx patch
<span class="prompt">$</span> govee-dmx run</code></pre>
<button class="copy" type="button" data-copy="cargo install govee-toolkit-dmx">Kopieren</button>
</div>

Art-Net ist das einzige Eingangsprotokoll der Bridge. Eine Leuchte, die sie
steuert, muss `lan` aktiviert haben.

[Die DMX-Anleitung]({{base}}docs/dmx/)

## MCP-Server {{version_mcp}} {{registry_mcp}} {#mcp-server}

Mit `govee-toolkit-mcp` liest ein KI-Assistent den Gerätekatalog und steuert die
Leuchten. Füge ihn deinem Assistenten hinzu, und `npx` lädt ihn beim ersten
Start:

::: tabs client
@tab Claude Code
<div class="terminal">
<pre><code><span class="prompt">$</span> claude mcp add govee -- npx -y govee-toolkit-mcp@latest</code></pre>
<button class="copy" type="button" data-copy="claude mcp add govee -- npx -y govee-toolkit-mcp@latest">Kopieren</button>
</div>
@tab Codex
<div class="terminal">
<pre><code><span class="prompt">$</span> codex mcp add govee -- npx -y govee-toolkit-mcp@latest</code></pre>
<button class="copy" type="button" data-copy="codex mcp add govee -- npx -y govee-toolkit-mcp@latest">Kopieren</button>
</div>
:::

[Die MCP-Anleitung]({{base}}docs/mcp/)
