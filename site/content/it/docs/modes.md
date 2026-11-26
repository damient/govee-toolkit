---
title: Modalità
slug: modes
order: 6
description: Wi-Fi, Bluetooth o cloud: da dove ogni strada raggiunge la tua luce, e cosa serve a ognuna.
source: 754a0b1129283a1e2ac0b73a9665404789b4437a
---

# Modalità

Un comando arriva alla tua luce lungo una di tre strade. Scegli tu quali strade
permettere, per ogni dispositivo.

| Modalità | Velocità | Raggiunge il dispositivo da |
| -------- | -------- | --------------------------- |
| {{badge_lan}} | la più veloce | lo stesso Wi-Fi |
| {{badge_ble}} | veloce | la portata Bluetooth |
| {{badge_cloud}} | la più lenta | ovunque ci sia internet |

## Cosa serve a ogni modalità {#what-each-mode-needs}

{{badge_lan}}: attiva "LAN Control" per il dispositivo nell'app Govee Home, e
metti il dispositivo e il tuo computer sulla stessa rete. Il Wi-Fi ospiti e
alcune reti mesh li separano. L'animazione dal vivo dei segmenti passa da questa modalità.

{{badge_ble}}: un adattatore Bluetooth sul tuo computer, e il dispositivo a
portata. Una connessione alla volta: un dispositivo connesso smette di farsi
vedere, quindi una scansione avviata mentre un altro controller tiene il
collegamento non trova niente.

{{badge_cloud}}: una chiave API Govee, e il dispositivo registrato su
quell'account. L'API ha un limite di richieste, e non trasporta animazioni.

## Dove sta la verità {#where-the-truth-lives}

Cosa raggiunge una modalità dipende dal modello. [La pagina dei dispositivi]({{base}}devices/)
dice a cosa risponde ognuno.
