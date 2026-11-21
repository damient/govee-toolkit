---
title: Serveur MCP
slug: mcp
order: 8
description: Laissez un assistant IA lire le catalogue des appareils et piloter vos lumières, par un serveur MCP local.
source: f9b43c0f13f88f7b6621ad144824feb2b0f4b1c8
---

# Serveur MCP

`govee-toolkit-mcp` est un serveur MCP local. Votre assistant lit le catalogue des
appareils à travers lui et pilote vos lumières via {{badge_lan}}, {{badge_ble}}
et {{badge_cloud}}, avec la même configuration que la commande `govee`.

## Ce qu'il vous faut {#what-you-need}

- Un assistant installé sur votre ordinateur qui accepte les serveurs MCP.
- Pour le pilotage, le `config.yaml` que décrit [Configuration]({{base}}docs/configure/).
  Les outils de lecture fonctionnent dès que le serveur démarre.

## Ajouter le serveur {#add-the-server}

Choisissez votre assistant. Chacun démarre le serveur sous le nom `govee`.

::: tabs client
@tab Claude Code
<div class="terminal">
<pre><code><span class="prompt">$</span> claude mcp add govee -- npx -y govee-toolkit-mcp@latest</code></pre>
<button class="copy" type="button" data-copy="claude mcp add govee -- npx -y govee-toolkit-mcp@latest">Copier</button>
</div>
@tab Claude Desktop
Ajoutez ceci à `claude_desktop_config.json`:

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
<button class="copy" type="button" data-copy="codex mcp add govee -- npx -y govee-toolkit-mcp@latest">Copier</button>
</div>
@tab Cursor
Ajoutez ceci à `.cursor/mcp.json`:

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
Ajoutez ceci à `.vscode/mcp.json`:

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

## Lui indiquer votre configuration {#point-it-at-your-configuration}

Le serveur lit deux variables :

- `GOVEE_CONFIG` nomme votre `config.yaml`. La valeur par défaut est
  `~/.config/govee-toolkit/config.yaml`.
- `GOVEE_API_KEY` est la clé d'API dont {{badge_cloud}} a besoin.

Donnez-les à votre assistant :

::: tabs client
@tab Claude Code
<div class="terminal">
<pre><code><span class="prompt">$</span> claude mcp add govee -e GOVEE_CONFIG=/home/me/.config/govee-toolkit/config.yaml -- npx -y govee-toolkit-mcp@latest</code></pre>
<button class="copy" type="button" data-copy="claude mcp add govee -e GOVEE_CONFIG=/home/me/.config/govee-toolkit/config.yaml -- npx -y govee-toolkit-mcp@latest">Copier</button>
</div>
@tab Claude Desktop
Ajoutez `env` à `claude_desktop_config.json`:

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
<button class="copy" type="button" data-copy="codex mcp add govee --env GOVEE_CONFIG=/home/me/.config/govee-toolkit/config.yaml -- npx -y govee-toolkit-mcp@latest">Copier</button>
</div>
@tab Cursor
Ajoutez `env` à `.cursor/mcp.json`:

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
Ajoutez `env` à `.vscode/mcp.json`:

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

Écrivez `GOVEE_CONFIG` sous forme de chemin absolu : le client choisit le
répertoire de travail. Le client garde son bloc `env` en clair : mettez donc
plutôt la clé d'API dans `~/.config/govee-toolkit/.env`.

## Ce que l'assistant peut faire {#what-the-assistant-can-do}

Les outils de lecture répondent à partir du catalogue livré avec le serveur, sur votre machine.

| Outil | Renvoie |
| ----- | ------- |
| `list_models` | Une ligne par modèle, avec la prise en charge de chaque mode |
| `describe_model` | La fiche complète d'un modèle, avec ses tables DMX |
| `get_dmx_profile` | Les tables de canaux DMX d'un modèle |
| `get_api` | Les méthodes qui servent chaque rôle, dans chaque langage |
| `read_doc` | Une page de la documentation |

Les outils de contrôle pilotent les lumières. Le SDK démarre au premier appel et
libère le port réseau 30 secondes après le dernier : la commande `govee` peut
ainsi faire un scan entre-temps.

| Outil | Action |
| ----- | ------ |
| `scan` | Trouve les appareils sur votre réseau |
| `list_known` | Liste les appareils trouvés, avec l'état de santé de chaque mode |
| `status` | Lit l'état d'un appareil |
| `set` | Règle l'alimentation, la luminosité, la couleur, la température de blanc, un segment, un dégradé ou le mode musique |
| `send` | Envoie une commande que déclare le fichier d'appareil |
| `stream` | Joue des trames sur les segments d'un ou plusieurs appareils, puis leur rend leurs couleurs |
| `identify` | Allume les appareils un par un, pour que vous voyiez lequel est lequel |
| `doctor` | Liste les problèmes de votre configuration |

Une cible est un appareil, un groupe de [Noms et groupes]({{base}}docs/targets/),
ou tous les appareils d'un modèle.

## Les modes restent explicites {#modes-stay-explicit}

`mode` fixe un appel sur un seul mode, que l'appareil doit avoir activé. Sinon,
l'ordre de votre `config.yaml` s'applique. Un appel qui échoue renvoie le code
d'erreur du SDK, par exemple `mode_not_enabled`, et reste sur le mode qu'il a utilisé.

Pour placer un appareil sur votre Wi-Fi, utilisez la commande `govee` ou une liaison.
Le mot de passe Wi-Fi reste alors entre vous et l'appareil.
