---
title: Risoluzione dei problemi
slug: troubleshooting
order: 9
description: Niente risponde alla scansione, un comando non fa niente, un valore viene rifiutato — cosa significa ognuno.
source: 29626cda78a98b9a2c684d72dc514da617657a12
---

# Risoluzione dei problemi

## Niente risponde a `govee scan` {#nothing-answers-govee-scan}

Controlla questi punti, in ordine:

1. **LAN Control è attivo.** App Govee Home, il tuo dispositivo, impostazioni,
   **LAN Control**. Ogni comando via Wi-Fi richiede quell'interruttore.
2. **Una sola rete.** Il tuo computer e la luce devono essere sulla stessa rete.
   Il Wi-Fi ospiti e alcune reti mesh li tengono separati, anche nella stessa casa.
3. **Il modello ha l'interruttore.** Non tutti i modelli ce l'hanno. Cercalo
   nella [pagina dei dispositivi]({{base}}devices/), che elenca ogni modello a
   cui Govee dà l'interruttore.
4. **Il tuo firewall.** La ricerca dei dispositivi usa il multicast su UDP. Un
   firewall che lo scarta rende invisibile ogni dispositivo.
5. **Un solo programma sulla porta 4002.** Le risposte tornano sulla porta UDP
   4002, e il sistema consegna ognuna a un solo programma lì. Un server MCP o
   `govee-dmx` in esecuzione può prenderle. `govee scan` avvisa quando vede
   quel programma: fermalo e rifai la scansione.

Esegui `govee doctor`. Segnala quello che riesce a vedere della tua
configurazione.

## L'interruttore LAN Control non c'è {#the-lan-control-switch-is-missing}

Prima controlla che il modello sia nella [pagina dei dispositivi]({{base}}devices/),
che elenca ogni modello a cui Govee dà l'interruttore. Se c'è e l'interruttore
non compare:

1. Controlla che la luce sia sul Wi-Fi nell'app Govee Home.
2. Scollega la luce, poi riaccendila.
3. Aspetta circa 30 minuti, poi guarda di nuovo le impostazioni.

Se l'interruttore continua a non comparire, contatta l'assistenza Govee.

## Un comando riporta successo e non succede niente {#a-command-reports-success-and-nothing-happens}

Il firmware ignora un comando che non conosce, e non risponde niente in
entrambi i casi. Un sondaggio fallito e una funzione non implementata sembrano
uguali dall'esterno.

Se il comando viene da `send`, il file del dispositivo può dichiarare qualcosa
che questo modello non supporta. Se viene da un verbo, segnalalo — è una vera
scoperta.

## Un valore viene rifiutato {#a-value-is-refused}

Il toolkit rifiuta un valore fuori dall'intervallo che il file del dispositivo
dichiara. Non lo corregge. Il firmware limiterebbe il valore in silenzio e
riporterebbe successo, e tu crederesti applicata un'impostazione che non lo è.

`govee describe <device>` stampa l'intervallo di ogni comando.

## Luminosità 0 non spegne la luce {#brightness-0-does-not-turn-the-light-off}

Su un modello verificato, il firmware limita `brightness: 0` a 1. Usa
`govee off` per spegnere il dispositivo.

## La luminosità per segmento non funziona via Wi-Fi {#per-segment-brightness-does-not-work-over-wi-fi}

Il canale dei segmenti su `lan` trasporta solo il colore. Lì la luminosità è
globale. `ble` trasporta la luminosità per zona, e anche `cloud` la trasporta
per zona. È una proprietà del trasporto, non una lacuna del codice.

## La scansione Bluetooth non trova niente {#the-bluetooth-scan-finds-nothing}

Un dispositivo Govee smette di farsi vedere appena qualcosa si connette a lui.
Se l'app Govee, o un altro programma, tiene la connessione, chiudilo e rifai la
scansione.

## Dopo un aggiornamento del firmware, il comportamento è cambiato {#after-a-firmware-update-behavior-changed}

Gli aggiornamenti del firmware cambiano il comportamento senza preavviso. Un
file del dispositivo registra il firmware su cui sono stati misurati i suoi
numeri. Verifica di nuovo dopo un aggiornamento, e segnala cosa è cambiato.

## Segnala un problema {#report-a-problem}

Apri una issue su [GitHub]({{repo}}/issues) con il numero del tuo modello, il
tuo sistema operativo e l'output di `govee doctor`. Oscura il nome della tua
rete e la tua chiave API.
