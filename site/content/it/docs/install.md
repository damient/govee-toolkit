---
title: Installazione
slug: install
order: 2
description: Il crate Rust, la riga di comando, il pacchetto Python, il pacchetto Node.js e il bridge DMX.
source: d2d3c23cd5e0fcc83f9d4544b1089bd06b6d3958
---

# Installazione

Il crate Rust è l'implementazione di riferimento. La riga di comando funziona
direttamente da un terminale. Python e Node.js si appoggiano allo stesso nucleo.
Il bridge DMX è un binario separato.

## Rust {#rust} {{version_rust}} {{registry_rust}}

Asincrono, su Tokio. La feature `lan` è attiva di default. `ble` e `cloud` sono
da attivare a scelta.

<div class="terminal">
<pre><code><span class="prompt">$</span> cargo add govee-toolkit
<span class="prompt">$</span> cargo add govee-toolkit --features ble,cloud</code></pre>
<button class="copy" type="button" data-copy="cargo add govee-toolkit">Copia</button>
</div>

```rust
use govee_toolkit::{Config, Filter, Govee};

let govee = Govee::start(Config::load()?).await?;
let devices = govee.devices(Filter::all(), None).await?;
devices.power(true).await;
```

Su Linux la feature `ble` raggiunge la radio tramite BlueZ su D-Bus, quindi la
compilazione richiede gli header di dbus (`apt install libdbus-1-dev`). macOS e
Windows compilano così come sono.

## Riga di comando {#command-line} {{version_cli}} {{registry_cli}}

Il binario è `govee`. Legge i file dei dispositivi tramite il crate.

<div class="terminal">
<pre><code><span class="prompt">$</span> cargo install govee-toolkit-cli
<span class="prompt">$</span> govee scan
<span class="prompt">$</span> govee on DEVICE
<span class="prompt">$</span> govee color DEVICE "#ff3d00"</code></pre>
<button class="copy" type="button" data-copy="cargo install govee-toolkit-cli">Copia</button>
</div>

[Ogni comando, con un esempio in ogni linguaggio]({{base}}docs/reference/)

## Python {#python} {{version_python}} {{registry_python}}

Un binding PyO3 sul nucleo Rust. L'API è solo `asyncio`, e la wheel contiene il
catalogo dei dispositivi, quindi `pip install` è tutta l'installazione.

<div class="terminal">
<pre><code><span class="prompt">$</span> pip install govee-toolkit</code></pre>
<button class="copy" type="button" data-copy="pip install govee-toolkit">Copia</button>
</div>

```python
from govee_toolkit import Govee

govee = await Govee.start()
devices = await govee.devices()
await devices.power(True)
await govee.close()
```

Le wheel sono `abi3` per Python 3.11 e successivi: Linux, macOS e Windows su
`x86_64` e `aarch64`, Linux su `armv7`, e musl su `x86_64`, `aarch64` e
`armv7`. Su qualsiasi altra piattaforma, pip compila dal repository, e questo
richiede una toolchain Rust.

## Node.js {#node-js} {{version_node}} {{registry_node}}

Un binding napi-rs sullo stesso nucleo, con i tipi TypeScript. Il pacchetto
contiene il catalogo dei dispositivi, quindi `npm install` è tutta
l'installazione.

<div class="terminal">
<pre><code><span class="prompt">$</span> npm install govee-toolkit</code></pre>
<button class="copy" type="button" data-copy="npm install govee-toolkit">Copia</button>
</div>

```javascript
import { Govee } from "govee-toolkit"

const govee = await Govee.start()
const devices = await govee.devices()
await devices.power(true)
await govee.close()
```

Il pacchetto richiede Node.js 20 o successivo. L'addon è distribuito per Linux
e Windows su `x86_64` e `aarch64`, e per macOS su `aarch64`. Un'altra
piattaforma compila dal repository, perché l'addon compila il catalogo dei
dispositivi da `devices/*.yaml`.

## Bridge DMX {#dmx-bridge} {{version_dmx}} {{registry_dmx}}

Il binario è `govee-dmx`. Riceve Art-Net dalla rete e scrive ogni frame su una
luce via `lan`.

<div class="terminal">
<pre><code><span class="prompt">$</span> cargo install govee-toolkit-dmx
<span class="prompt">$</span> govee-dmx patch
<span class="prompt">$</span> govee-dmx run</code></pre>
<button class="copy" type="button" data-copy="cargo install govee-toolkit-dmx">Copia</button>
</div>

Art-Net è l'unico protocollo di ingresso che il bridge gestisce. Una luce che
comanda deve avere `lan` abilitata.

[La guida DMX]({{base}}docs/dmx/)

## Server MCP {#mcp-server} {{version_mcp}} {{registry_mcp}}

`govee-toolkit-mcp` permette a un assistente IA di leggere il catalogo dei
dispositivi e di comandare le luci. Aggiungilo al tuo assistente, e `npx` lo
scarica al primo avvio:

::: tabs client
@tab Claude Code
<div class="terminal">
<pre><code><span class="prompt">$</span> claude mcp add govee -- npx -y govee-toolkit-mcp@latest</code></pre>
<button class="copy" type="button" data-copy="claude mcp add govee -- npx -y govee-toolkit-mcp@latest">Copia</button>
</div>
@tab Codex
<div class="terminal">
<pre><code><span class="prompt">$</span> codex mcp add govee -- npx -y govee-toolkit-mcp@latest</code></pre>
<button class="copy" type="button" data-copy="codex mcp add govee -- npx -y govee-toolkit-mcp@latest">Copia</button>
</div>
:::

[La guida MCP]({{base}}docs/mcp/)
