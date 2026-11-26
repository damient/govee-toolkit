---
title: Modi
slug: modes
order: 6
description: WLAN, Bluetooth oder die Cloud: von wo aus jeder Weg deine Leuchte erreicht, und was jeder braucht.
source: 754a0b1129283a1e2ac0b73a9665404789b4437a
---

# Modi

Ein Befehl erreicht deine Leuchte über einen von drei Wegen. Du wählst für jedes
Gerät, welche Wege erlaubt sind.

| Modus | Geschwindigkeit | Er erreicht das Gerät von |
| ----- | --------------- | ------------------------- |
| {{badge_lan}} | am schnellsten | demselben WLAN |
| {{badge_ble}} | schnell | Bluetooth-Reichweite |
| {{badge_cloud}} | am langsamsten | überall mit Internet |

## Was jeder Modus braucht {#what-each-mode-needs}

{{badge_lan}}: schalte „LAN Control“ für das Gerät in der Govee Home App ein,
und bring das Gerät und deinen Computer ins selbe Netzwerk. Gast-WLAN und manche
Mesh-Systeme trennen sie. Die Live-Animation der Segmente läuft über diesen Modus.

{{badge_ble}}: ein Bluetooth-Adapter an deinem Computer, und das Gerät in
Reichweite. Eine Verbindung zur Zeit: Ein verbundenes Gerät sendet keine
Advertisements mehr. Ein Scan, während ein anderer Controller die Verbindung
hält, findet deshalb nichts.

{{badge_cloud}}: ein Govee-API-Schlüssel, und das Gerät, registriert auf dieses
Konto. Die API ist gedrosselt und überträgt keine Animation.

## Wo die Wahrheit steht {#where-the-truth-lives}

Was ein Modus erreicht, hängt vom Modell ab. [Die Geräteseite]({{base}}devices/)
sagt, was jedes Modell beantwortet.
