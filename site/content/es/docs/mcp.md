---
title: Servidor MCP
slug: mcp
order: 8
description: Deja que un asistente de IA lea el catálogo de dispositivos y controle tus luces, a través de un servidor MCP local.
source: f9b43c0f13f88f7b6621ad144824feb2b0f4b1c8
---

# Servidor MCP

`govee-toolkit-mcp` es un servidor MCP local. Tu asistente lee el catálogo de
dispositivos a través de él y controla tus luces por {{badge_lan}},
{{badge_ble}} y {{badge_cloud}}, con la misma configuración que el comando
`govee`.

## Qué necesitas {#what-you-need}

- Un asistente instalado en tu ordenador que acepte servidores MCP.
- Para el control, el `config.yaml` que describe
  [Configurar]({{base}}docs/configure/). Las herramientas de lectura funcionan
  en cuanto arranca el servidor.

## Añade el servidor {#add-the-server}

Elige tu asistente. Cada uno arranca el servidor con el nombre `govee`.

::: tabs client
@tab Claude Code
<div class="terminal">
<pre><code><span class="prompt">$</span> claude mcp add govee -- npx -y govee-toolkit-mcp@latest</code></pre>
<button class="copy" type="button" data-copy="claude mcp add govee -- npx -y govee-toolkit-mcp@latest">Copiar</button>
</div>
@tab Claude Desktop
Añade esto a `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "govee": {
      "command": "npx",
      "args": ["-y", "govee-toolkit-mcp@latest"]
    }
  }
}
```
@tab Codex
<div class="terminal">
<pre><code><span class="prompt">$</span> codex mcp add govee -- npx -y govee-toolkit-mcp@latest</code></pre>
<button class="copy" type="button" data-copy="codex mcp add govee -- npx -y govee-toolkit-mcp@latest">Copiar</button>
</div>
@tab Cursor
Añade esto a `.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "govee": {
      "command": "npx",
      "args": ["-y", "govee-toolkit-mcp@latest"]
    }
  }
}
```
@tab VS Code
Añade esto a `.vscode/mcp.json`:

```json
{
  "servers": {
    "govee": {
      "type": "stdio",
      "command": "npx",
      "args": ["-y", "govee-toolkit-mcp@latest"]
    }
  }
}
```
:::

## Apúntalo a tu configuración {#point-it-at-your-configuration}

El servidor lee dos variables:

- `GOVEE_CONFIG` indica tu `config.yaml`. El valor por defecto es
  `~/.config/govee-toolkit/config.yaml`.
- `GOVEE_API_KEY` es la clave de API que necesita {{badge_cloud}}.

Pásaselas a tu asistente:

::: tabs client
@tab Claude Code
<div class="terminal">
<pre><code><span class="prompt">$</span> claude mcp add govee -e GOVEE_CONFIG=/home/me/.config/govee-toolkit/config.yaml -- npx -y govee-toolkit-mcp@latest</code></pre>
<button class="copy" type="button" data-copy="claude mcp add govee -e GOVEE_CONFIG=/home/me/.config/govee-toolkit/config.yaml -- npx -y govee-toolkit-mcp@latest">Copiar</button>
</div>
@tab Claude Desktop
Añade `env` a `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "govee": {
      "command": "npx",
      "args": ["-y", "govee-toolkit-mcp@latest"],
      "env": {
        "GOVEE_CONFIG": "/home/me/.config/govee-toolkit/config.yaml"
      }
    }
  }
}
```
@tab Codex
<div class="terminal">
<pre><code><span class="prompt">$</span> codex mcp add govee --env GOVEE_CONFIG=/home/me/.config/govee-toolkit/config.yaml -- npx -y govee-toolkit-mcp@latest</code></pre>
<button class="copy" type="button" data-copy="codex mcp add govee --env GOVEE_CONFIG=/home/me/.config/govee-toolkit/config.yaml -- npx -y govee-toolkit-mcp@latest">Copiar</button>
</div>
@tab Cursor
Añade `env` a `.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "govee": {
      "command": "npx",
      "args": ["-y", "govee-toolkit-mcp@latest"],
      "env": {
        "GOVEE_CONFIG": "/home/me/.config/govee-toolkit/config.yaml"
      }
    }
  }
}
```
@tab VS Code
Añade `env` a `.vscode/mcp.json`:

```json
{
  "servers": {
    "govee": {
      "type": "stdio",
      "command": "npx",
      "args": ["-y", "govee-toolkit-mcp@latest"],
      "env": {
        "GOVEE_CONFIG": "/home/me/.config/govee-toolkit/config.yaml"
      }
    }
  }
}
```
:::

Escribe `GOVEE_CONFIG` como ruta absoluta: el cliente elige el directorio de
trabajo. El cliente guarda su bloque `env` en texto plano, así que pon la clave
de API en `~/.config/govee-toolkit/.env`.

## Lo que puede hacer el asistente {#what-the-assistant-can-do}

Las herramientas de lectura responden desde el catálogo que incluye el
servidor, en tu equipo.

| Herramienta | Devuelve |
| ----------- | -------- |
| `list_models` | Una fila por modelo, con el soporte de cada modo |
| `describe_model` | El registro completo de un modelo, con sus tablas DMX |
| `get_dmx_profile` | Las tablas de canales DMX de un modelo |
| `get_api` | Los métodos que sirven a cada función, en cada lenguaje |
| `read_doc` | Una página de la documentación |

Las herramientas de control manejan las luces. El SDK arranca en la primera
llamada y libera el puerto de red 30 segundos después de la última, así el
comando `govee` puede escanear entre medias.

| Herramienta | Hace |
| ----------- | ---- |
| `scan` | Encuentra los dispositivos de tu red |
| `list_known` | Lista los dispositivos encontrados, con el estado de cada modo |
| `status` | Lee el estado de un dispositivo |
| `set` | Ajusta el encendido, el brillo, el color, la temperatura de blanco, un segmento, un degradado o el modo música |
| `send` | Envía un comando que declara el archivo del dispositivo |
| `stream` | Reproduce fotogramas en los segmentos de uno o varios dispositivos, y después les devuelve los colores que tenían antes |
| `identify` | Enciende los dispositivos uno a uno, para que veas cuál es cuál |
| `doctor` | Lista los problemas de tu configuración |

Un destino es un dispositivo, un grupo de [Nombres y grupos]({{base}}docs/targets/),
o todos los dispositivos de un modelo.

## Los modos siguen siendo explícitos {#modes-stay-explicit}

`mode` fija una llamada a un modo, que el dispositivo debe tener habilitado. Si
no, se aplica el orden de tu `config.yaml`. Una llamada que falla devuelve el
código de error del SDK, como `mode_not_enabled`, y se queda en el modo que usó.

Para conectar un dispositivo a tu Wi-Fi, usa el comando `govee` o un binding.
Así la contraseña de la Wi-Fi queda entre tú y el dispositivo.
