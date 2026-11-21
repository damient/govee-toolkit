---
title: Installation
slug: install
order: 2
description: La crate Rust, la ligne de commande, le paquet Python, le paquet Node.js et le pont DMX.
source: d2d3c23cd5e0fcc83f9d4544b1089bd06b6d3958
---

# Installation

La crate Rust est l'implémentation de référence. La ligne de commande fonctionne
directement depuis un terminal. Python et Node.js se lient à ce même cœur. Le pont
DMX est un binaire distinct.

## Rust {#rust} {{version_rust}} {{registry_rust}}

Asynchrone, sur Tokio. La feature `lan` est active par défaut. `ble` et `cloud` sont
à activer.

<div class="terminal">
<pre><code><span class="prompt">$</span> cargo add govee-toolkit
<span class="prompt">$</span> cargo add govee-toolkit --features ble,cloud</code></pre>
<button class="copy" type="button" data-copy="cargo add govee-toolkit">Copier</button>
</div>

```rust
use govee_toolkit::{Config, Filter, Govee};

let govee = Govee::start(Config::load()?).await?;
let devices = govee.devices(Filter::all(), None).await?;
devices.power(true).await;
```

Sous Linux, la feature `ble` atteint la radio par BlueZ sur D-Bus : la compilation
a donc besoin des en-têtes dbus (`apt install libdbus-1-dev`). macOS et Windows
compilent tels quels.

## Ligne de commande {#command-line} {{version_cli}} {{registry_cli}}

Le binaire s'appelle `govee`. Il lit les fichiers d'appareil par la crate.

<div class="terminal">
<pre><code><span class="prompt">$</span> cargo install govee-toolkit-cli
<span class="prompt">$</span> govee scan
<span class="prompt">$</span> govee on DEVICE
<span class="prompt">$</span> govee color DEVICE "#ff3d00"</code></pre>
<button class="copy" type="button" data-copy="cargo install govee-toolkit-cli">Copier</button>
</div>

[Chaque commande, avec un exemple dans chaque langage]({{base}}docs/reference/)

## Python {#python} {{version_python}} {{registry_python}}

Une liaison PyO3 sur le cœur Rust. L'API est uniquement `asyncio`, et le wheel
contient le catalogue des appareils : `pip install` suffit à tout installer.

<div class="terminal">
<pre><code><span class="prompt">$</span> pip install govee-toolkit</code></pre>
<button class="copy" type="button" data-copy="pip install govee-toolkit">Copier</button>
</div>

```python
from govee_toolkit import Govee

govee = await Govee.start()
devices = await govee.devices()
await devices.power(True)
await govee.close()
```

Les wheels sont `abi3` pour Python 3.11 et plus : Linux, macOS et Windows sur
`x86_64` et `aarch64`, Linux sur `armv7`, et musl sur `x86_64`, `aarch64` et
`armv7`. Sur toute autre plateforme, pip compile depuis le dépôt, ce qui
demande une chaîne d'outils Rust.

## Node.js {#node-js} {{version_node}} {{registry_node}}

Une liaison napi-rs sur le même cœur, avec les types TypeScript. Le paquet
contient le catalogue des appareils : `npm install` suffit à tout installer.

<div class="terminal">
<pre><code><span class="prompt">$</span> npm install govee-toolkit</code></pre>
<button class="copy" type="button" data-copy="npm install govee-toolkit">Copier</button>
</div>

```javascript
import { Govee } from "govee-toolkit"

const govee = await Govee.start()
const devices = await govee.devices()
await devices.power(true)
await govee.close()
```

Le paquet demande Node.js 20 ou plus récent. L'extension native est livrée pour
Linux et Windows sur `x86_64` et `aarch64`, et pour macOS sur `aarch64`. Une autre
plateforme compile depuis le dépôt, car l'extension compile le catalogue des
appareils à partir de `devices/*.yaml`.

## Pont DMX {#dmx-bridge} {{version_dmx}} {{registry_dmx}}

Le binaire s'appelle `govee-dmx`. Il reçoit l'Art-Net du réseau et écrit chaque
trame vers une lumière via `lan`.

<div class="terminal">
<pre><code><span class="prompt">$</span> cargo install govee-toolkit-dmx
<span class="prompt">$</span> govee-dmx patch
<span class="prompt">$</span> govee-dmx run</code></pre>
<button class="copy" type="button" data-copy="cargo install govee-toolkit-dmx">Copier</button>
</div>

Art-Net est le seul protocole d'entrée que le pont accepte. Une lumière qu'il pilote
doit avoir `lan` activé.

[Le guide DMX]({{base}}docs/dmx/)

## Serveur MCP {#mcp-server} {{version_mcp}} {{registry_mcp}}

`govee-toolkit-mcp` permet à un assistant IA de lire le catalogue des appareils et de
piloter les lumières. Ajoutez-le à votre assistant, et `npx` le télécharge au premier
démarrage :

::: tabs client
@tab Claude Code
<div class="terminal">
<pre><code><span class="prompt">$</span> claude mcp add govee -- npx -y govee-toolkit-mcp@latest</code></pre>
<button class="copy" type="button" data-copy="claude mcp add govee -- npx -y govee-toolkit-mcp@latest">Copier</button>
</div>
@tab Codex
<div class="terminal">
<pre><code><span class="prompt">$</span> codex mcp add govee -- npx -y govee-toolkit-mcp@latest</code></pre>
<button class="copy" type="button" data-copy="codex mcp add govee -- npx -y govee-toolkit-mcp@latest">Copier</button>
</div>
:::

[Le guide MCP]({{base}}docs/mcp/)
