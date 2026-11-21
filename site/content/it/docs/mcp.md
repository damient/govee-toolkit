---
title: Server MCP
slug: mcp
order: 8
description: Permetti a un assistente IA di leggere il catalogo dei dispositivi e di comandare le tue luci, tramite un server MCP locale.
source: f9b43c0f13f88f7b6621ad144824feb2b0f4b1c8
---

# Server MCP

`govee-toolkit-mcp` è un server MCP locale. Il tuo assistente legge tramite lui
il catalogo dei dispositivi e comanda le tue luci via {{badge_lan}},
{{badge_ble}} e {{badge_cloud}}, con la stessa configurazione del comando
`govee`.

## Cosa ti serve {#what-you-need}

- Un assistente installato sul tuo computer che accetta server MCP.
- Per il controllo, il `config.yaml` descritto in
  [Configurazione]({{base}}docs/configure/). Gli strumenti di lettura funzionano
  appena il server parte.

## Aggiungi il server {#add-the-server}

Scegli il tuo assistente. Ognuno avvia il server con il nome `govee`.

::: tabs client
@tab Claude Code
<div class="terminal">
<pre><code><span class="prompt">$</span> claude mcp add govee -- npx -y govee-toolkit-mcp@latest</code></pre>
<button class="copy" type="button" data-copy="claude mcp add govee -- npx -y govee-toolkit-mcp@latest">Copia</button>
</div>
@tab Claude Desktop
Aggiungi questo a `claude_desktop_config.json`:

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
<button class="copy" type="button" data-copy="codex mcp add govee -- npx -y govee-toolkit-mcp@latest">Copia</button>
</div>
@tab Cursor
Aggiungi questo a `.cursor/mcp.json`:

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
Aggiungi questo a `.vscode/mcp.json`:

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

## Collegalo alla tua configurazione {#point-it-at-your-configuration}

Il server legge due variabili:

- `GOVEE_CONFIG` indica il tuo `config.yaml`. Il valore predefinito è
  `~/.config/govee-toolkit/config.yaml`.
- `GOVEE_API_KEY` è la chiave API che serve a {{badge_cloud}}.

Passale al tuo assistente:

::: tabs client
@tab Claude Code
<div class="terminal">
<pre><code><span class="prompt">$</span> claude mcp add govee -e GOVEE_CONFIG=/home/me/.config/govee-toolkit/config.yaml -- npx -y govee-toolkit-mcp@latest</code></pre>
<button class="copy" type="button" data-copy="claude mcp add govee -e GOVEE_CONFIG=/home/me/.config/govee-toolkit/config.yaml -- npx -y govee-toolkit-mcp@latest">Copia</button>
</div>
@tab Claude Desktop
Aggiungi `env` a `claude_desktop_config.json`:

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
<button class="copy" type="button" data-copy="codex mcp add govee --env GOVEE_CONFIG=/home/me/.config/govee-toolkit/config.yaml -- npx -y govee-toolkit-mcp@latest">Copia</button>
</div>
@tab Cursor
Aggiungi `env` a `.cursor/mcp.json`:

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
Aggiungi `env` a `.vscode/mcp.json`:

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

Scrivi `GOVEE_CONFIG` come percorso assoluto: è il client a scegliere la
directory di lavoro. Il client conserva il suo blocco `env` in chiaro, quindi
metti invece la chiave API in `~/.config/govee-toolkit/.env`.

## Cosa può fare l'assistente {#what-the-assistant-can-do}

Gli strumenti di lettura rispondono dal catalogo distribuito con il server, sul
tuo computer.

| Strumento | Restituisce |
| --------- | ----------- |
| `list_models` | Una riga per modello, con il supporto di ogni modalità |
| `describe_model` | La scheda completa di un modello, con le sue tabelle DMX |
| `get_dmx_profile` | Le tabelle dei canali DMX di un modello |
| `get_api` | I metodi che servono ogni ruolo, in ogni linguaggio |
| `read_doc` | Una pagina della documentazione |

Gli strumenti di controllo comandano le luci. L'SDK parte alla prima chiamata, e
libera la porta di rete 30 secondi dopo l'ultima, così il comando `govee` può
fare una scansione nel frattempo.

| Strumento | Cosa fa |
| --------- | ------- |
| `scan` | Trova i dispositivi sulla tua rete |
| `list_known` | Elenca i dispositivi trovati, con lo stato di salute di ogni modalità |
| `status` | Legge lo stato di un dispositivo |
| `set` | Imposta accensione, luminosità, colore, temperatura del bianco, un segmento, una sfumatura o la modalità musica |
| `send` | Invia un comando dichiarato nel file del dispositivo |
| `stream` | Riproduce frame sui segmenti di uno o più dispositivi, poi riporta i colori che avevano prima |
| `identify` | Accende i dispositivi uno per uno, così vedi quale è quale |
| `doctor` | Elenca i problemi della tua configurazione |

Un target è un dispositivo, un gruppo di [Nomi e gruppi]({{base}}docs/targets/),
o ogni dispositivo di un modello.

## Le modalità restano esplicite {#modes-stay-explicit}

`mode` vincola una chiamata a una modalità, che il dispositivo deve abilitare.
Altrimenti si applica l'ordine del tuo `config.yaml`. Una chiamata che fallisce
restituisce il codice di errore dell'SDK, come `mode_not_enabled`, e resta sulla
modalità che ha usato.

Per collegare un dispositivo al tuo Wi-Fi, usa il comando `govee` o un binding.
Così la password del Wi-Fi resta tra te e il dispositivo.
