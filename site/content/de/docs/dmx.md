---
title: DMX
slug: dmx
order: 7
description: Steuere deine Leuchten per DMX von einem Lichtpult, so wie eine Show jedes andere Fixture steuert.
source: 7361202b2432615a85bfa555df6791f68a2134c2
---

# DMX

`govee-dmx` nimmt das Art-Net, das dein Pult schon sendet, und schreibt es über
{{badge_lan}} an jede Leuchte. Deine Leuchten werden zu Fixtures der Show.

## Was du brauchst {#what-you-need}

- Eine Leuchte, die {{badge_lan}} erreicht. [Die Geräteseite]({{base}}devices/)
  sagt, welche Modelle das sind.
- Ein Pult, einen Medienserver oder eine Show-Software, die Art-Net sendet.
- Eine Patch-Datei, die sagt, welche Leuchte auf welche Kanäle antwortet.

## Eine Kanalbelegung wählen {#pick-a-channel-layout}

Jede Leuchte antwortet auf eine von drei Belegungen. Ein Pult nennt das eine
Personality.

| Belegung | Kanäle | Was du bekommst |
| -------- | ------ | --------------- |
| `full` | 6 | eine Farbe für die ganze Leuchte |
| `segment` | 3 + 3 pro Zone | eine Farbe pro Zone |
| `pixel` | 3 + 3 pro LED | eine Farbe pro LED |

Kanal 1 ist immer der Dimmer, Kanal 2 der Modus und Kanal 3 die
Farbtemperatur. So funktioniert ein Cue von einem Modell zum nächsten. Das Weiß
gilt für die ganze Leuchte, unabhängig von der Belegung. Ein Modell bietet nur
die Belegungen, die seine Hardware trägt. Jede Modellseite zeigt ihre Tabellen,
Kanal für Kanal — [die H61A0]({{base}}devices/H61A0/#dmx) ist ein Beispiel.

## Ein Fixture patchen {#patch-a-fixture}

Zwei Befehle richten das Rig für dich ein.

```sh
govee-dmx patch      # find the lights and write the patch file
govee-dmx identify   # light one fixture at a time
```

`patch` sucht die Leuchten in deinem Netzwerk und gibt jeder eine freie
Startadresse. Jede Adresse, die du schon gesetzt hast, bleibt unverändert. Du
kannst den Befehl also erneut ausführen, nachdem du eine Leuchte hinzugefügt
hast.

`identify` geht das Rig durch. Der Befehl schaltet jedes Fixture aus, lässt sie
dann eines nach dem anderen leuchten und nennt die Zeile der Patch-Datei, die
jedes steuert. So beschriftest du die Fixtures am Pult.

Der erste Kanal ist immer der Dimmer. Bei 0 schaltet er die Leuchte aus. Ein
Blackout am Pult schaltet also das ganze Rig dunkel.

## Das Rig betreiben {#run-the-rig}

```sh
govee-dmx run        # receive Art-Net and drive the lights
```

`run` wartet auf Art-Net und schreibt jeden Frame an die Leuchten, die die
Patch-Datei nennt. Dein Pult listet den Node, sobald er auf einen Poll antwortet,
und du patchst ihn dort wie jeden anderen Node. Mit `--scan` sucht der Befehl
zuerst die Leuchten. So bringt ein einziger Befehl ein Rig von null bis zu einem
Node, den das Pult sieht.

`--dry-run` gibt jedes Paket aus und was jedes Fixture daraus liest, und lässt
jede Leuchte, wie sie ist. So prüfst du einen Patch, bevor sich das Rig bewegt.

Der Lauf schaltet zu Beginn jede Leuchte aus, das Rig startet also dunkel. Wenn
eine Leuchte aus dem Netzwerk fällt, machen die anderen weiter, und der Lauf
meldet es.
