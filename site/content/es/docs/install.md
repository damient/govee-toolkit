---
title: Instalar
slug: install
order: 2
description: El crate de Rust, la línea de comandos, el paquete de Python, el paquete de Node.js y el puente DMX.
source: d2d3c23cd5e0fcc83f9d4544b1089bd06b6d3958
---

# Instalar

El crate de Rust es la implementación de referencia. La línea de comandos
funciona directamente desde una terminal. Python y Node.js se enlazan con ese
mismo núcleo. El puente DMX es un binario aparte.

## Rust {#rust} {{version_rust}} {{registry_rust}}

Asíncrono, sobre Tokio. La feature `lan` está activa por defecto. `ble` y
`cloud` se activan a elección.

<div class="terminal">
<pre><code><span class="prompt">$</span> cargo add govee-toolkit
<span class="prompt">$</span> cargo add govee-toolkit --features ble,cloud</code></pre>
<button class="copy" type="button" data-copy="cargo add govee-toolkit">Copiar</button>
</div>

```rust
use govee_toolkit::{Config, Filter, Govee};

let govee = Govee::start(Config::load()?).await?;
let devices = govee.devices(Filter::all(), None).await?;
devices.power(true).await;
```

En Linux, la feature `ble` llega a la radio a través de BlueZ por D-Bus, así
que la compilación necesita las cabeceras de dbus (`apt install libdbus-1-dev`).
macOS y Windows compilan tal cual.

## Línea de comandos {#command-line} {{version_cli}} {{registry_cli}}

El binario es `govee`. Lee los archivos de dispositivo a través del crate.

<div class="terminal">
<pre><code><span class="prompt">$</span> cargo install govee-toolkit-cli
<span class="prompt">$</span> govee scan
<span class="prompt">$</span> govee on DEVICE
<span class="prompt">$</span> govee color DEVICE "#ff3d00"</code></pre>
<button class="copy" type="button" data-copy="cargo install govee-toolkit-cli">Copiar</button>
</div>

[Todos los comandos, con un ejemplo en cada lenguaje]({{base}}docs/reference/)

## Python {#python} {{version_python}} {{registry_python}}

Un binding PyO3 sobre el núcleo de Rust. La API es solo `asyncio`, y el wheel
incluye el catálogo de dispositivos, así que `pip install` es toda la
instalación.

<div class="terminal">
<pre><code><span class="prompt">$</span> pip install govee-toolkit</code></pre>
<button class="copy" type="button" data-copy="pip install govee-toolkit">Copiar</button>
</div>

```python
from govee_toolkit import Govee

govee = await Govee.start()
devices = await govee.devices()
await devices.power(True)
await govee.close()
```

Los wheels son `abi3` para Python 3.11 y posteriores: Linux, macOS y Windows
en `x86_64` y `aarch64`, Linux en `armv7`, y musl en `x86_64`, `aarch64` y
`armv7`. En cualquier otra plataforma, pip compila desde el repositorio, lo que
necesita una toolchain de Rust.

## Node.js {#node-js} {{version_node}} {{registry_node}}

Un binding napi-rs sobre el mismo núcleo, con tipos de TypeScript. El paquete
incluye el catálogo de dispositivos, así que `npm install` es toda la
instalación.

<div class="terminal">
<pre><code><span class="prompt">$</span> npm install govee-toolkit</code></pre>
<button class="copy" type="button" data-copy="npm install govee-toolkit">Copiar</button>
</div>

```javascript
import { Govee } from "govee-toolkit"

const govee = await Govee.start()
const devices = await govee.devices()
await devices.power(true)
await govee.close()
```

El paquete necesita Node.js 20 o posterior. El addon se distribuye para Linux
y Windows en `x86_64` y `aarch64`, y para macOS en `aarch64`. Otra plataforma
compila desde el repositorio, porque el addon compila el catálogo de
dispositivos a partir de `devices/*.yaml`.

## Puente DMX {#dmx-bridge} {{version_dmx}} {{registry_dmx}}

El binario es `govee-dmx`. Recibe Art-Net de la red y escribe cada trama en una
luz por `lan`.

<div class="terminal">
<pre><code><span class="prompt">$</span> cargo install govee-toolkit-dmx
<span class="prompt">$</span> govee-dmx patch
<span class="prompt">$</span> govee-dmx run</code></pre>
<button class="copy" type="button" data-copy="cargo install govee-toolkit-dmx">Copiar</button>
</div>

Art-Net es el único protocolo de entrada que admite el puente. Una luz que
controla debe tener `lan` habilitado.

[La guía de DMX]({{base}}docs/dmx/)

## Servidor MCP {#mcp-server} {{version_mcp}} {{registry_mcp}}

`govee-toolkit-mcp` permite a un asistente de IA leer el catálogo de
dispositivos y controlar las luces. Añádelo a tu asistente, y `npx` lo descarga
en el primer arranque:

::: tabs client
@tab Claude Code
<div class="terminal">
<pre><code><span class="prompt">$</span> claude mcp add govee -- npx -y govee-toolkit-mcp@latest</code></pre>
<button class="copy" type="button" data-copy="claude mcp add govee -- npx -y govee-toolkit-mcp@latest">Copiar</button>
</div>
@tab Codex
<div class="terminal">
<pre><code><span class="prompt">$</span> codex mcp add govee -- npx -y govee-toolkit-mcp@latest</code></pre>
<button class="copy" type="button" data-copy="codex mcp add govee -- npx -y govee-toolkit-mcp@latest">Copiar</button>
</div>
:::

[La guía de MCP]({{base}}docs/mcp/)
