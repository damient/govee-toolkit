---
title: DMX
slug: dmx
order: 7
description: Comanda le tue luci da una console luci in DMX, come uno spettacolo comanda ogni altro apparecchio.
source: 7361202b2432615a85bfa555df6791f68a2134c2
---

# DMX

`govee-dmx` prende l'Art-Net che la tua console già invia e lo scrive su ogni
luce via {{badge_lan}}. Le tue luci diventano apparecchi dello spettacolo.

## Cosa ti serve {#what-you-need}

- Una luce che {{badge_lan}} raggiunge. [La pagina dei dispositivi]({{base}}devices/)
  dice quali modelli lo fanno.
- Una console, un media server o un'applicazione per spettacoli che invia Art-Net.
- Un file di patch che dice quale luce risponde a quali canali.

## Scegli un layout dei canali {#pick-a-channel-layout}

Ogni luce risponde a uno di tre layout. Una console lo chiama personality.

| Layout | Canali | Cosa ottieni |
| ------ | ------ | ------------ |
| `full` | 6 | un colore per tutta la luce |
| `segment` | 3 + 3 per zona | un colore per zona |
| `pixel` | 3 + 3 per LED | un colore per LED |

Il canale 1 è sempre il dimmer, il canale 2 la modalità e il canale 3 la
temperatura del bianco, così un cue passa da un modello all'altro. Il bianco
copre tutta la luce, qualunque sia il layout. Un modello offre solo i layout che
il suo hardware supporta. Ogni pagina di modello stampa le sue tabelle, canale
per canale — [l'H61A0]({{base}}devices/H61A0/#dmx) è un esempio.

## Fai il patch di un apparecchio {#patch-a-fixture}

Due comandi preparano l'impianto per te.

```sh
govee-dmx patch      # find the lights and write the patch file
govee-dmx identify   # light one fixture at a time
```

`patch` cerca le luci sulla tua rete e dà a ognuna un indirizzo di partenza
libero. Non tocca nessun indirizzo che hai già impostato, quindi puoi eseguirlo
di nuovo dopo aver aggiunto una luce.

`identify` percorre l'impianto. Spegne tutti gli apparecchi, poi li accende uno
per uno e nomina la riga del file di patch che comanda ognuno. È così che
etichetti gli apparecchi sulla console.

Il primo canale è sempre il dimmer. A 0 spegne la luce, quindi un blackout
sulla console spegne l'impianto.

## Avvia l'impianto {#run-the-rig}

```sh
govee-dmx run        # receive Art-Net and drive the lights
```

`run` ascolta l'Art-Net e scrive ogni frame sulle luci indicate nel file di
patch. La tua console elenca il nodo appena risponde a un poll, e lì ne fai il
patch come per qualsiasi altro nodo. Aggiungi `--scan` per cercare prima le
luci, così un solo comando porta un impianto da zero a un nodo che la console
vede.

`--dry-run` stampa ogni pacchetto e cosa ne legge ogni apparecchio, e lascia
ogni luce com'è. È così che verifichi un patch prima che l'impianto si muova.

L'avvio spegne tutte le luci, quindi l'impianto parte al buio. Quando una luce
esce dalla rete, le altre continuano e l'esecuzione lo segnala.
