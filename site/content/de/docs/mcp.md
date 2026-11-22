---
title: MCP-Server
slug: mcp
order: 8
description: Lass einen KI-Assistenten den Gerätekatalog lesen und deine Leuchten steuern, über einen lokalen MCP-Server.
source: 92c6411f45f2e8e5d5006a6c8cf7e45127325edf
---

# MCP-Server

`govee-toolkit-mcp` ist ein lokaler MCP-Server. Dein Assistent liest darüber den
Gerätekatalog und steuert deine Leuchten über {{badge_lan}}, {{badge_ble}} und
{{badge_cloud}}, mit derselben Konfiguration wie der Befehl `govee`.

## Was du brauchst {#what-you-need}

- Einen auf deinem Computer installierten Assistenten, der MCP-Server annimmt.
- Für die Steuerung die `config.yaml`, die [Konfiguration]({{base}}docs/configure/)
  beschreibt. Die Lese-Tools funktionieren, sobald der Server startet.

## Den Server hinzufügen {#add-the-server}

Wähle deinen Assistenten. Jeder startet den Server unter dem Namen `govee`.

::: tabs client
@tab Claude Code
<div class="terminal">
<pre><code><span class="prompt">$</span> claude mcp add govee -- npx -y govee-toolkit-mcp@latest</code></pre>
<button class="copy" type="button" data-copy="claude mcp add govee -- npx -y govee-toolkit-mcp@latest">Kopieren</button>
</div>
@tab Claude Desktop
Füge das in `claude_desktop_config.json` ein:

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
<button class="copy" type="button" data-copy="codex mcp add govee -- npx -y govee-toolkit-mcp@latest">Kopieren</button>
</div>
@tab Cursor
Füge das in `.cursor/mcp.json` ein:

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
Füge das in `.vscode/mcp.json` ein:

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

## Auf deine Konfiguration zeigen {#point-it-at-your-configuration}

Der Server liest zwei Variablen:

- `GOVEE_CONFIG` nennt deine `config.yaml`. Der Standard ist
  `config.yaml` im [Konfigurationsordner]({{base}}docs/configure/).
- `GOVEE_API_KEY` ist der API-Schlüssel, den {{badge_cloud}} braucht.

Gib sie deinem Assistenten:

::: tabs client
@tab Claude Code
<div class="terminal">
<pre><code><span class="prompt">$</span> claude mcp add govee -e GOVEE_CONFIG=/home/me/govee.yaml -- npx -y govee-toolkit-mcp@latest</code></pre>
<button class="copy" type="button" data-copy="claude mcp add govee -e GOVEE_CONFIG=/home/me/govee.yaml -- npx -y govee-toolkit-mcp@latest">Kopieren</button>
</div>
@tab Claude Desktop
Füge `env` in `claude_desktop_config.json` ein:

```json
{
  "mcpServers": {
    "govee": {
      "command": "npx",
      "args": ["-y", "govee-toolkit-mcp@latest"],
      "env": {
        "GOVEE_CONFIG": "/home/me/govee.yaml"
      }
    }
  }
}
```
@tab Codex
<div class="terminal">
<pre><code><span class="prompt">$</span> codex mcp add govee --env GOVEE_CONFIG=/home/me/govee.yaml -- npx -y govee-toolkit-mcp@latest</code></pre>
<button class="copy" type="button" data-copy="codex mcp add govee --env GOVEE_CONFIG=/home/me/govee.yaml -- npx -y govee-toolkit-mcp@latest">Kopieren</button>
</div>
@tab Cursor
Füge `env` in `.cursor/mcp.json` ein:

```json
{
  "mcpServers": {
    "govee": {
      "command": "npx",
      "args": ["-y", "govee-toolkit-mcp@latest"],
      "env": {
        "GOVEE_CONFIG": "/home/me/govee.yaml"
      }
    }
  }
}
```
@tab VS Code
Füge `env` in `.vscode/mcp.json` ein:

```json
{
  "servers": {
    "govee": {
      "type": "stdio",
      "command": "npx",
      "args": ["-y", "govee-toolkit-mcp@latest"],
      "env": {
        "GOVEE_CONFIG": "/home/me/govee.yaml"
      }
    }
  }
}
```
:::

Schreib `GOVEE_CONFIG` als absoluten Pfad: Der Client wählt das
Arbeitsverzeichnis. Der Client speichert seinen `env`-Block im Klartext. Leg den
API-Schlüssel deshalb stattdessen in `credentials.env` im Konfigurationsordner ab.

## Was der Assistent kann {#what-the-assistant-can-do}

Lese-Tools antworten aus dem Katalog, der mit dem Server kommt, auf deinem
Rechner.

| Tool | Liefert |
| ---- | ------- |
| `list_models` | Eine Zeile pro Modell, mit der Unterstützung jedes Modus |
| `describe_model` | Den vollständigen Eintrag eines Modells, mit seinen DMX-Tabellen |
| `get_dmx_profile` | Die DMX-Kanaltabellen eines Modells |
| `get_api` | Die Methoden für jede Rolle, in jeder Sprache |
| `read_doc` | Eine Seite der Dokumentation |

Steuer-Tools steuern die Leuchten. Das SDK startet beim ersten Aufruf und gibt
den Netzwerkport 30 Sekunden nach dem letzten frei. So kann der Befehl `govee`
dazwischen scannen.

| Tool | Macht |
| ---- | ----- |
| `scan` | Findet die Geräte in deinem Netzwerk |
| `list_known` | Listet die gefundenen Geräte, mit dem Zustand jedes Modus |
| `status` | Liest den Zustand eines Geräts |
| `set` | Setzt Ein/Aus, Helligkeit, Farbe, Farbtemperatur, ein Segment, einen Verlauf oder den Musikmodus |
| `send` | Sendet einen Befehl, den die Gerätedatei deklariert |
| `stream` | Spielt Frames auf den Segmenten eines oder mehrerer Geräte ab und stellt dann ihre Farben wieder her |
| `identify` | Lässt die Geräte eines nach dem anderen leuchten, damit du siehst, welches welches ist |
| `doctor` | Listet die Probleme in deiner Konfiguration |

Ein Ziel ist ein Gerät, eine Gruppe aus [Namen und Gruppen]({{base}}docs/targets/)
oder jedes Gerät eines Modells.

## Modi bleiben explizit {#modes-stay-explicit}

`mode` legt einen Aufruf auf einen Modus fest, den das Gerät aktiviert haben
muss. Sonst gilt die Reihenfolge in deiner `config.yaml`. Ein Aufruf, der
fehlschlägt, gibt den Fehlercode des SDK zurück, etwa `mode_not_enabled`, und
bleibt bei dem Modus, den er genutzt hat.

Um ein Gerät in dein WLAN zu bringen, nutze den Befehl `govee` oder ein Binding.
Das WLAN-Passwort bleibt dann zwischen dir und dem Gerät.
