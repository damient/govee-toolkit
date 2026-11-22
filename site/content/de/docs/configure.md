---
title: Konfiguration
slug: configure
order: 3
description: Die config.yaml, die deine Geräte benennt, die .env-Datei, die das Toolkit liest, die WLAN-Zugangsdaten, mit denen ein Gerät deinem Netzwerk beitritt, und der API-Schlüssel, den der Cloud-Modus braucht.
source: ac7a5ca370cd6102f6679a62f9e74a3d695e661d
---

# Konfiguration

Das Toolkit liest zwei Dateien. `config.yaml` benennt deine Geräte und legt
fest, welchen Modus jedes nutzt. `.env` enthält die Zugangsdaten.

`lan` und `ble` senden Befehle ohne weitere Einrichtung. Lies diese Seite, wenn
du ein Gerät in dein WLAN bringst oder wenn du `cloud` aktivierst.

## Die Datei `config.yaml` {#the-config-yaml-file}

Solange du diese Datei nicht schreibst, nutzt jedes Gerät `lan`. Lege sie selbst
im Konfigurationsordner deines Systems an, oder zeige mit `GOVEE_CONFIG`
oder `--config` auf einen anderen Pfad.

| System | Konfigurationsordner |
| --- | --- |
| macOS | `~/Library/Application Support/govee-toolkit` |
| Linux | `$XDG_CONFIG_HOME/govee-toolkit`, sonst `~/.config/govee-toolkit` |
| Windows | `%APPDATA%\govee-toolkit` |

Der Schlüssel jedes Geräts ist die MAC-Adresse, die es meldet. `govee scan`
listet sie auf:

<div class="terminal">
<pre><code><span class="prompt">$</span> govee scan
AA:BB:CC:DD:EE:FF  H6159  -  [lan]
11:22:33:44:55:66  H619A  -  [lan]</code></pre>
<button class="copy" type="button" data-copy="govee scan">Kopieren</button>
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

`modes:` listet die Modi, die ein Gerät nutzen darf, die erste Wahl zuerst. Die
[Seite zu den Modi]({{base}}docs/modes/) beschreibt jeden Modus, und
[Namen und Gruppen]({{base}}docs/targets/) beschreibt `name:` und `groups:`.

Das Toolkit lehnt einen Schlüssel ab, den es nicht kennt. Ein Tippfehler schlägt
also beim Start fehl. `govee doctor` meldet, was an der Datei falsch ist.

## Die Datei `.env` {#the-env-file}

Eine Variable, die du exportierst, gilt so lange wie die Shell. Eine
`.env`-Datei gilt auch in einer neuen. Das Toolkit liest die erste Datei, die es
findet, in dieser Reihenfolge:

1. Die Datei, die `--env-file` nennt, oder die Datei, die `GOVEE_ENV_FILE`
   nennt. Fehlt diese Datei, ist das ein Fehler, weil jemand genau diese Datei
   verlangt hat.
2. `.env` im Verzeichnis, in dem du arbeitest, dann in jedem Verzeichnis darüber.
   Die Suche endet nach dem Verzeichnis, das `.git` enthält, nach deinem
   Home-Verzeichnis oder an der Wurzel des Dateisystems.
3. `.env` im Konfigurationsordner.

`credentials.env` im Konfigurationsordner kommt danach und liefert die
Namen, die keine andere Datei setzt. Studio schreibt den API-Schlüssel dorthin,
wenn du eine Sicherung importierst.

Das Toolkit liest nur die `GOVEE_*`-Namen einer `.env`. Eine Datei kann also
mehreren Projekten dienen. Eine Variable, die schon in deiner Umgebung steht,
hat Vorrang vor der Datei, und `govee --no-env` überspringt die Datei.

`govee doctor` sagt, aus welcher Datei die Variablen stammen:

<div class="terminal">
<pre><code><span class="prompt">$</span> govee doctor
variables from `/home/you/lights/.env`
no problem found</code></pre>
<button class="copy" type="button" data-copy="govee doctor">Kopieren</button>
</div>

## WLAN {#wi-fi}

{{badge_lan}} — ein Gerät antwortet über `lan`, sobald es in deinem eigenen WLAN
ist. Die Govee Home App bringt es dorthin, und `govee provision` macht dasselbe
über `ble`. Zwei Variablen liefern den Netzwerknamen und das Passwort. So bleibt
das Passwort aus deinem Shell-Verlauf heraus:

```bash
GOVEE_WIFI_SSID=my-network
GOVEE_WIFI_PASSWORD=the password
```

<div class="terminal">
<pre><code><span class="prompt">$</span> govee provision AA:BB:CC:DD:EE:FF</code></pre>
<button class="copy" type="button" data-copy="govee provision AA:BB:CC:DD:EE:FF">Kopieren</button>
</div>

`--ssid` und `--password` nehmen dieselben Werte auf der Kommandozeile an, und
`--open` tritt einem offenen Netzwerk bei. Der Befehl braucht einen Build mit
Bluetooth: `cargo install govee-toolkit-cli --features ble`.

Vier Dinge, die du wissen musst, bevor du ihn ausführst:

- Nutze ein 2,4-GHz-Netzwerk. Kein Govee-Gerät tritt einem 5-GHz-Netzwerk bei.
- Das Passwort wird im Klartext übertragen, ohne Schlüsselaustausch. Alles in
  Bluetooth-Reichweite kann es lesen, während der Befehl läuft.
- Das Gerät bestätigt die Übertragung, und die Meldung sagt, ob es die
  Zugangsdaten angenommen hat. Sie sagt nicht, ob das Gerät dem Netzwerk
  beigetreten ist: Prüfe das Netzwerk danach.
- Schließe zuerst die Govee Home App. Sie hält die eine Verbindung, die der
  Funkchip annimmt.

## Der API-Schlüssel {#the-api-key}

{{badge_cloud}} — dieser Modus läuft über die API von Govee und braucht einen
Schlüssel. [Erste Schritte]({{base}}docs/start/) sagt, wie du ihn in der App
beantragst. Gib ihn dem Toolkit als `GOVEE_API_KEY`:

```bash
GOVEE_API_KEY=7c41b9d2-5a6e-4f08-9b31-2d84ec55af10
```

Schreib den Schlüssel nie in `config.yaml`. Diese Datei landet in
Fehlerberichten. Das Toolkit protokolliert den Schlüssel nie und schreibt ihn nie
in den Gerätecache.
