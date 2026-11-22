---
title: Configuration
slug: configure
order: 3
description: Le config.yaml qui nomme vos appareils, le fichier .env que le toolkit lit, les identifiants Wi-Fi dont un appareil a besoin pour rejoindre votre réseau, et la clé d'API que demande le mode cloud.
source: ac7a5ca370cd6102f6679a62f9e74a3d695e661d
---

# Configuration

Le toolkit lit deux fichiers. `config.yaml` nomme vos appareils et indique le
mode de chacun. `.env` contient les identifiants.

`lan` et `ble` envoient les commandes telles quelles. Lisez cette page quand vous
placez un appareil sur votre réseau Wi-Fi, ou quand vous activez `cloud`.

## Le fichier `config.yaml` {#the-config-yaml-file}

Tant que vous n'écrivez pas ce fichier, chaque appareil utilise `lan`. Écrivez-le
vous-même dans le dossier de configuration de votre système, ou faites pointer
`GOVEE_CONFIG` ou `--config` vers un autre chemin.

| Système | Dossier de configuration |
| --- | --- |
| macOS | `~/Library/Application Support/govee-toolkit` |
| Linux | `$XDG_CONFIG_HOME/govee-toolkit`, sinon `~/.config/govee-toolkit` |
| Windows | `%APPDATA%\govee-toolkit` |

Chaque appareil a pour clé l'adresse MAC qu'il annonce. `govee scan` les liste :

<div class="terminal">
<pre><code><span class="prompt">$</span> govee scan
AA:BB:CC:DD:EE:FF  H6159  -  [lan]
11:22:33:44:55:66  H619A  -  [lan]</code></pre>
<button class="copy" type="button" data-copy="govee scan">Copier</button>
</div>

```yaml
defaults:
  modes: [lan]

devices:
  "AA:BB:CC:DD:EE:FF":
    name: "kitchen"
  "11:22:33:44:55:66":
    modes: [lan, ble]
```

`modes:` liste les modes qu'un appareil peut utiliser, le premier choix en premier. La
[page des modes]({{base}}docs/modes/) décrit chaque mode, et
[Noms et groupes]({{base}}docs/targets/) décrit `name:` et `groups:`.

Le toolkit refuse une clé qu'il ne connaît pas : une faute de frappe échoue donc au
démarrage. `govee doctor` signale ce qui ne va pas dans le fichier.

## Le fichier `.env` {#the-env-file}

Une variable que vous exportez dure autant que le shell. Un fichier `.env` survit à
un nouveau shell. Le toolkit lit le premier fichier qu'il trouve, dans cet ordre :

1. Le fichier que nomme `--env-file`, ou le fichier que nomme `GOVEE_ENV_FILE`. Un
   fichier absent est ici une erreur, car quelqu'un a demandé ce fichier.
2. `.env` dans le répertoire où vous travaillez, puis dans chaque répertoire
   parent. La recherche s'arrête après le répertoire qui contient `.git`, après
   votre répertoire personnel, ou à la racine du système de fichiers.
3. `.env` dans le dossier de configuration.

`credentials.env`, dans le dossier de configuration, vient après eux et
fournit les noms qu'aucun autre fichier ne définit. Studio y écrit la clé d'API
quand vous importez une sauvegarde.

Le toolkit lit uniquement les noms `GOVEE_*` d'un `.env` : un seul fichier peut donc
servir plusieurs projets. Une variable déjà présente dans votre environnement
l'emporte sur le fichier, et `govee --no-env` ignore le fichier.

`govee doctor` indique de quel fichier viennent les variables :

<div class="terminal">
<pre><code><span class="prompt">$</span> govee doctor
variables from `/home/you/lights/.env`
no problem found</code></pre>
<button class="copy" type="button" data-copy="govee doctor">Copier</button>
</div>

## Wi-Fi {#wi-fi}

{{badge_lan}} — un appareil répond en `lan` dès qu'il est sur votre propre réseau Wi-Fi.
L'application Govee Home l'y place, et `govee provision` fait de même en `ble`. Deux
variables fournissent le nom du réseau et le mot de passe : le mot de passe reste
ainsi hors de l'historique de votre shell.

```bash
GOVEE_WIFI_SSID=my-network
GOVEE_WIFI_PASSWORD=the password
```

<div class="terminal">
<pre><code><span class="prompt">$</span> govee provision AA:BB:CC:DD:EE:FF</code></pre>
<button class="copy" type="button" data-copy="govee provision AA:BB:CC:DD:EE:FF">Copier</button>
</div>

`--ssid` et `--password` prennent les mêmes valeurs en ligne de commande, et
`--open` rejoint un réseau ouvert. La commande demande une version compilée avec le
Bluetooth : `cargo install govee-toolkit-cli --features ble`.

Quatre points à connaître avant de la lancer :

- Utilisez un réseau 2,4 GHz. Aucun appareil Govee ne rejoint un réseau 5 GHz.
- Le mot de passe circule en clair, sans échange de clés. Tout ce qui est à portée
  Bluetooth pendant l'exécution de la commande peut le lire.
- L'appareil accuse réception du transfert, et le rapport indique s'il a accepté
  les identifiants. Il n'indique pas que l'appareil a rejoint le réseau :
  vérifiez le réseau ensuite.
- Fermez d'abord l'application Govee Home. Elle tient la seule connexion que la
  radio accepte.

## La clé d'API {#the-api-key}

{{badge_cloud}} — ce mode passe par l'API de Govee et demande une clé.
[Commencer ici]({{base}}docs/start/) explique comment la demander dans l'application.
Donnez-la au toolkit sous le nom `GOVEE_API_KEY` :

```bash
GOVEE_API_KEY=7c41b9d2-5a6e-4f08-9b31-2d84ec55af10
```

Ne mettez jamais la clé dans `config.yaml`. Ce fichier finit dans des rapports de
bug. Le toolkit n'écrit jamais la clé dans ses journaux ni dans le cache des
appareils.
