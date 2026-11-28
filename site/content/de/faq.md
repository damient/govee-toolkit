---
title: Häufige Fragen
description: Was Govee Toolkit ist, welche Govee-Leuchten es erreicht, und was du brauchst, um sie über WLAN, über Bluetooth oder über die Cloud zu steuern.
source: e290c00ed743d71ab8f736a7316468768037d409
---

## Was ist Govee Toolkit? {#what-is-govee-toolkit} <!-- menu: Worum es geht -->

Govee Toolkit steuert deine Govee-Leuchten von deinem eigenen Computer aus. Es
schaltet sie ein und aus und stellt Helligkeit, Farbe und Farbtemperatur ein.
Auf einem LED-Streifen färbt es jede Zone einzeln. Es ist kostenlos und Open
Source, unter der MIT-Lizenz.

## Ist es ein offizielles Govee-Produkt? {#is-it-an-official-govee-product} <!-- menu: Offiziell oder nicht -->

Nein. Es ist ein unabhängiges Projekt, nicht mit Govee verbunden, nicht von
Govee gesponsert oder unterstützt. Der Name „Govee“ bezeichnet die Leuchten,
mit denen es funktioniert.

## Funktioniert meine Govee-Leuchte damit? {#does-my-govee-light-work-with-it} <!-- menu: Kompatible Leuchten -->

[Die Geräteseite]({{base}}devices/) führt jedes Modell auf, das das Toolkit
kennt, und was jedes über WLAN, über Bluetooth und über die Cloud beantwortet.
Ein Modell, das noch niemand geprüft hat, zeigt ein Fragezeichen: ungetestet,
und das heißt noch lange nicht defekt.

## Mein Modell steht nicht auf der Liste. Was kann ich tun? {#my-model-is-not-on-the-list-what-can-i-do} <!-- menu: Ein fehlendes Modell -->

Zwei Wege bringen es dorthin. Schick uns die Leuchte, und wir fügen sie
kostenlos hinzu, oder folge der Anleitung und füge sie selbst hinzu.
[Gerät hinzufügen]({{base}}devices/add/) beschreibt beide Wege.

## Muss ich programmieren können? {#do-i-need-to-know-how-to-code} <!-- menu: Programmierkenntnisse -->

Nein. Die Kommandozeile nimmt einen kurzen Befehl pro Aktion an, etwa
`govee on` oder `govee color`. [Erste Schritte]({{base}}docs/start/) kommt ganz
ohne Code aus. Entwickler nutzen dieselbe Engine aus Rust, Python oder Node.js.

## Funktioniert es ohne Internet? {#does-it-work-without-the-internet} <!-- menu: Ohne Internet -->

Ja. Über WLAN und über Bluetooth geht jeder Befehl von deinem Computer direkt an
die Leuchte, in deinem eigenen Netzwerk. Die Cloud ist ein dritter Modus, den
du nur aktivierst, wenn du deine Leuchten von überall erreichen willst.

## WLAN, Bluetooth oder Cloud: Was wähle ich? {#wi-fi-bluetooth-or-cloud-which-one-do-i-pick} <!-- menu: WLAN, Bluetooth oder Cloud -->

- **WLAN** ist am schnellsten. Die Leuchte und dein Computer teilen ein
  Netzwerk.
- **Bluetooth** erreicht eine Leuchte in Reichweite, direkt von deinem
  Computer.
- **Cloud** erreicht deine Leuchten von überall mit Internetzugang.

Du wählst die Modi für jede Leuchte. [Modi]({{base}}docs/modes/) sagt, was
jeder braucht.

## Was ist LAN Control, und wie schalte ich es ein? {#what-is-lan-control-and-how-do-i-turn-it-on} <!-- menu: LAN Control -->

LAN Control ist der Schalter, mit dem dein eigenes Netzwerk die Leuchte über
WLAN erreicht. Öffne die Govee Home App, wähle deine Leuchte, öffne ihre
Einstellungen und schalte **LAN Control** ein. Das machst du einmal pro
Leuchte. [Die Geräteseite]({{base}}devices/) führt jedes Modell auf, dem Govee
den Schalter gibt.

## Brauche ich ein Govee-Konto oder einen API-Schlüssel? {#do-i-need-a-govee-account-or-an-api-key} <!-- menu: Konto und API-Schlüssel -->

Nur für den Cloud-Modus. WLAN und Bluetooth funktionieren, sobald die Leuchte
an ist. Für die Cloud beantragst du einen Schlüssel in der Govee Home App:
Profil, dann Einstellungen, dann **Apply for API key**.

## Kann ich die Govee Home App weiter nutzen? {#can-i-keep-using-the-govee-home-app} <!-- menu: Die Govee Home App -->

Ja. Über Bluetooth hält eine Leuchte eine Verbindung zur Zeit: Schließe die
Govee Home App, wenn sich das Toolkit über Bluetooth verbindet.

## Kann ich eine neue Leuchte ohne die Govee Home App in mein WLAN bringen? {#can-i-put-a-new-light-on-my-wi-fi-without-the-govee-home-app} <!-- menu: WLAN-Einrichtung -->

Ja. `govee provision` sendet den Netzwerknamen und das Passwort über Bluetooth
an die Leuchte. Nutze ein 2,4-GHz-Netzwerk: Govee-Leuchten treten keinem
5-GHz-Netzwerk bei. [Konfiguration]({{base}}docs/configure/#wi-fi) gibt die
Details.

## Auf welchen Computern läuft es? {#which-computers-does-it-run-on} <!-- menu: Unterstützte Systeme -->

Linux, macOS und Windows. Das Python-Paket enthält außerdem Builds für Linux
auf ARM-Boards, `aarch64` und `armv7`.

## Kann ich jedes Segment eines LED-Streifens steuern? {#can-i-control-each-segment-of-a-light-strip} <!-- menu: Segmente -->

Ja, auf einem Modell mit Segmenten. Sprich eine Zone, einen Bereich oder den
ganzen Streifen mit einem Befehl an. Jede Modellseite nennt ihre Zonenzahl.

## Können meine Leuchten Musik oder meinem Bildschirm folgen? {#can-my-lights-follow-music-or-my-screen} <!-- menu: Musik und Bildschirm -->

Das Toolkit streamt Farben Frame für Frame in Echtzeit an einen Streifen, über
WLAN. Dein eigenes Programm, oder eines, das du wählst, liefert die Frames: aus
Musik, vom Bildschirm oder von einem Sensor. Ein Modell mit Mikrofon spielt
auch seine eigenen Musikeffekte: Seine Modellseite nennt die Modi, die sie
übertragen.

## Kann ich die Szenen meiner Leuchte abspielen? {#can-i-play-the-scenes-of-my-light} <!-- menu: Szenen -->

Ja, auf einem Modell, dessen Seite „Eine Szene abspielen“ nennt. `govee scenes
fetch <Modell>` speichert die Szenenbibliothek des Modells einmal, ohne Konto.
`govee scene <Gerät> "<Name>"` spielt dann eine Szene über ihren Namen oder
ihren Code. Du kannst auch eine eigene Zeichnung auf dem Gerät speichern, und
das Gerät spielt sie weiter.

## Kann ich meine Leuchten per DMX steuern? {#can-i-control-my-lights-with-dmx} <!-- menu: DMX -->

Ja. Die DMX-Bridge, `govee-dmx`, empfängt Art-Net von einem Lichtpult, einem
Medienserver oder einer Show-Software, und jede Leuchte wird zu einem
DMX-Fixture. [DMX]({{base}}docs/dmx/) erklärt die Einrichtung.

## Kann ein KI-Assistent meine Leuchten steuern? {#can-an-ai-assistant-control-my-lights} <!-- menu: KI-Assistent -->

Ja. Der MCP-Server, `govee-toolkit-mcp`, läuft auf deinem Rechner und gibt
einem Assistenten den Gerätekatalog und die Steuerung: Scan, Status, Ein/Aus,
Helligkeit, Farbe, Segmente und Szenen. [MCP-Server]({{base}}docs/mcp/) erklärt die
Einrichtung.

## Funktioniert es mit Home Assistant, Homebridge oder Matter? {#does-it-work-with-home-assistant-homebridge-or-matter} <!-- menu: Home Assistant, Matter -->

Als Nächstes auf der Roadmap: eine Desktop-App, dann Home Assistant, Homebridge
und Matter. [Die Roadmap]({{repo}}/blob/main/docs/roadmap.md) richtet sich
danach, was Leute sich wünschen.

## Meine Leuchte antwortet nicht. Was prüfe ich? {#my-light-does-not-answer-what-do-i-check} <!-- menu: Fehlerbehebung -->

Prüfe zuerst diese Punkte:

1. LAN Control ist für die Leuchte eingeschaltet.
2. Die Leuchte und dein Computer sind im selben Netzwerk. Gast-WLAN trennt sie.
3. Deine Firewall lässt Multicast über UDP durch.

[Fehlerbehebung]({{base}}docs/troubleshooting/) deckt den Rest ab.

## Wo stelle ich eine Frage oder melde ein Problem? {#where-do-i-ask-a-question-or-report-a-problem} <!-- menu: Kontakt -->

Öffne ein Issue auf [GitHub]({{repo}}/issues) mit deiner Modellnummer und
deinem Betriebssystem. Für eine Leuchte, die du uns schicken willst, schreib an
[contact@gvetk.com](mailto:contact@gvetk.com).
