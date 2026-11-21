---
title: Namen und Gruppen
slug: targets
order: 4
description: Gib einem Gerät einen Namen, fasse mehrere Geräte in einer Gruppe zusammen, und steuere sie über diesen Namen statt über eine MAC-Adresse.
source: 363675951b17dbdc5e606db0fbbadd2c283c3a6f
---

# Namen und Gruppen

Eine MAC-Adresse ist schwer zu tippen und noch schwerer zu merken. Gib jedem
Gerät in `config.yaml` einen Namen, fasse Geräte, die zusammengehören, in einer
Gruppe zusammen, und jeder Befehl nimmt diese Namen anstelle der MAC-Adresse an.

## Ein Gerät benennen {#name-a-device}

`name:` gibt einem Gerät einen Namen. `groups:` setzt es in eine oder mehrere
Gruppen:

```yaml
devices:
  "AA:BB:CC:DD:EE:FF":
    name: "kitchen"
    groups: [ambient]
  "11:22:33:44:55:66":
    name: "desk"
    groups: [ambient, office]
  "22:33:44:55:66:77":
    name: "shelf"
    groups: [office]
```

Eine Gruppe existiert, sobald ein Gerät sie trägt. Ein Name und eine Gruppe
passen nur vollständig und ohne Rücksicht auf Groß- und Kleinschreibung. So
findet `Kitchen` den Namen `kitchen`, und `kit` findet nichts.

## Eine Gruppe steuern {#drive-a-group}

Ein Befehl an eine Gruppe geht gleichzeitig an jedes Mitglied:

<div class="terminal">
<pre><code><span class="prompt">$</span> govee on ambient
<span class="prompt">$</span> govee brightness ambient 40</code></pre>
<button class="copy" type="button" data-copy="govee brightness ambient 40">Kopieren</button>
</div>

Jedes Mitglied antwortet für sich:

- Wenn ein Mitglied scheitert, machen die anderen weiter. Der Fehler geht an
  stderr, und der Befehl endet mit dem Code des ersten Mitglieds, das gescheitert
  ist.
- Jedes Mitglied nutzt seine eigenen Modi. `--mode` beschränkt jedes Mitglied
  auf diesen Modus. Ein Mitglied, das ihn nicht aktiviert hat, scheitert, und
  die anderen machen weiter.
- Jedes Mitglied prüft den Wert gegen seinen eigenen Bereich. Eine Helligkeit
  kann für ein Mitglied im Bereich liegen und für ein anderes außerhalb.

## Ein Ziel schreiben {#write-a-target}

Ein Ziel ist das, was du dort eintippst, wo ein Befehl ein Gerät erwartet. Es
hat eine von vier Formen:

| Ziel | Es bezeichnet |
| ---- | ------------- |
| `AA:BB:CC:DD:EE:FF` | das Gerät mit dieser Identität |
| `kitchen` | das Gerät, das die Konfiguration `kitchen` nennt |
| `ambient` | jedes Gerät in der Gruppe `ambient` |
| `H6159` | jedes bekannte Gerät dieses Modells |

Ein Befehl, der eine Leuchte steuert, etwa `on` oder `brightness`, nimmt eine
Identität, einen Namen oder eine Gruppe an. `govee devices` und
`govee identify` nehmen beliebig viele Ziele an, auch ein Modell.

Ein Befehl, der eine Leuchte steuert, liest den Namen oder die Gruppe direkt aus
`config.yaml`. `govee devices` listet, was schon bekannt ist. Scanne also zuerst:
Ein Modell, ein Name und eine Gruppe passen nur auf die Geräte, die ein Scan
gefunden hat.
