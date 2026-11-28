---
title: Fehlerbehebung
slug: troubleshooting
order: 9
description: Nichts antwortet auf den Scan, ein Befehl bewirkt nichts, ein Wert wird abgelehnt — was jeder Fall bedeutet.
source: 3efc01557d239fe5fdbfe28919096d20636a6be3
---

# Fehlerbehebung

## Nichts antwortet auf `govee scan` {#nothing-answers-govee-scan}

Prüfe diese Punkte, in dieser Reihenfolge:

1. **LAN Control ist eingeschaltet.** Govee Home App, dein Gerät,
   Einstellungen, **LAN Control**. Jeder Befehl über WLAN braucht diesen
   Schalter.
2. **Ein Netzwerk.** Dein Computer und die Leuchte müssen im selben Netzwerk
   sein. Gast-WLAN und manche Mesh-Systeme trennen sie, auch in einem Haus.
3. **Das Modell hat den Schalter.** Nicht jedes Modell hat ihn. Sieh auf
   [der Geräteseite]({{base}}devices/) nach. Sie führt jedes Modell auf, dem
   Govee den Schalter gibt.
4. **Deine Firewall.** Die Suche nutzt Multicast über UDP. Eine Firewall, die
   es verwirft, macht jedes Gerät unsichtbar.
5. **Ein Programm auf Port 4002.** Antworten kommen an UDP-Port 4002 zurück,
   und das System gibt jede an ein einziges Programm dort weiter. Ein laufender
   MCP-Server oder `govee-dmx` kann sie abfangen. `govee scan` warnt, wenn es
   dieses Programm sieht: Beende es und scanne erneut.

Führe `govee doctor` aus. Es meldet, was es von deiner Konfiguration sehen kann.

## Der Schalter LAN Control fehlt {#the-lan-control-switch-is-missing}

Prüfe zuerst, ob das Modell auf [der Geräteseite]({{base}}devices/) steht. Sie
führt jedes Modell auf, dem Govee den Schalter gibt. Steht es dort und der
Schalter erscheint nicht:

1. Prüfe, ob die Leuchte in der Govee Home App im WLAN ist.
2. Zieh den Stecker der Leuchte und schalte sie dann wieder ein.
3. Warte etwa 30 Minuten und sieh dann noch einmal in die Einstellungen.

Erscheint der Schalter immer noch nicht, wende dich an den Govee-Support.

## Ein Befehl meldet Erfolg, und nichts passiert {#a-command-reports-success-and-nothing-happens}

Die Firmware ignoriert einen Befehl, den sie nicht kennt, und antwortet in
beiden Fällen nichts. Eine fehlgeschlagene Abfrage und eine nicht implementierte
Funktion sehen von außen gleich aus.

Kommt der Befehl von `send`, deklariert die Gerätedatei vielleicht etwas, das
dieses Modell nicht hat. Kommt er von einem Verb, melde ihn — das ist ein
echter Befund.

## Ein Wert wird abgelehnt {#a-value-is-refused}

Das Toolkit lehnt einen Wert außerhalb des Bereichs ab, den die Gerätedatei
deklariert. Es passt ihn nicht an. Die Firmware würde den Wert stillschweigend
begrenzen und Erfolg melden, und du würdest glauben, eine Einstellung sei
übernommen, obwohl sie es nicht ist.

`govee describe <device>` gibt den Bereich jedes Befehls aus.

## Helligkeit 0 schaltet die Leuchte nicht aus {#brightness-0-does-not-turn-the-light-off}

Auf einem verifizierten Modell begrenzt die Firmware `brightness: 0` auf 1.
Nutze `govee off`, um das Gerät auszuschalten.

## Helligkeit pro Segment funktioniert nicht über WLAN {#per-segment-brightness-does-not-work-over-wi-fi}

Der Segmentkanal auf `lan` überträgt nur Farbe. Die Helligkeit ist dort global.
`ble` überträgt die Helligkeit pro Zone, und `cloud` überträgt sie pro Zone. Das
ist eine Eigenschaft der Übertragung, keine Lücke im Code.

## Eine Szene wird nicht gefunden {#a-scene-is-not-found}

`govee scene` liest die Szenenbibliothek des Modells aus einem Cache auf
deinem Computer.

- `scene_list_missing`: Führe einmal `govee scenes fetch <Modell>` aus, dann
  versuch es noch einmal.
- `unknown_scene`: `govee scenes list <Modell>` zeigt die Namen und die Codes.
- `ambiguous_scene`: Der Name passt auf mehr als eine Szene. Gib einen vollen
  Namen aus der Fehlermeldung an, oder einen Code.

## Der Bluetooth-Scan findet nichts {#the-bluetooth-scan-finds-nothing}

Ein Govee-Gerät sendet keine Advertisements mehr, sobald sich etwas mit ihm
verbindet. Hält die Govee Home App oder ein anderes Programm die Verbindung,
schließe es und scanne erneut.

## Nach einem Firmware-Update hat sich das Verhalten geändert {#after-a-firmware-update-behavior-changed}

Firmware-Updates ändern das Verhalten ohne Ankündigung. Eine Gerätedatei hält
fest, auf welcher Firmware ihre Werte gemessen wurden. Prüfe nach einem Update
erneut, und melde, was sich geändert hat.

## Ein Problem melden {#report-a-problem}

Öffne ein Issue auf [GitHub]({{repo}}/issues) mit deiner Modellnummer, deinem
Betriebssystem und der Ausgabe von `govee doctor`. Entferne deinen
Netzwerknamen und deinen API-Schlüssel.
