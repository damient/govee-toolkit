---
title: Nomi e gruppi
slug: targets
order: 4
description: Dai un nome a un dispositivo, metti più dispositivi in un gruppo, e comandali con quel nome invece che con un indirizzo MAC.
source: 363675951b17dbdc5e606db0fbbadd2c283c3a6f
---

# Nomi e gruppi

Un indirizzo MAC è difficile da scrivere e ancora più da ricordare. Dai un nome
a ogni dispositivo in `config.yaml`, metti in un gruppo i dispositivi che vanno
insieme, e ogni comando accetta quei nomi al posto dell'indirizzo MAC.

## Dai un nome a un dispositivo {#name-a-device}

`name:` dà un nome a un dispositivo. `groups:` lo mette in uno o più gruppi:

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

Un gruppo esiste appena un dispositivo lo porta. Un nome e un gruppo
corrispondono solo per intero e ignorano maiuscole e minuscole, quindi `Kitchen`
trova `kitchen` e `kit` non trova niente.

## Comanda un gruppo {#drive-a-group}

Un comando inviato a un gruppo va a tutti i membri contemporaneamente:

<div class="terminal">
<pre><code><span class="prompt">$</span> govee on ambient
<span class="prompt">$</span> govee brightness ambient 40</code></pre>
<button class="copy" type="button" data-copy="govee brightness ambient 40">Copia</button>
</div>

Ogni membro risponde per conto suo:

- Quando un membro fallisce, gli altri vanno avanti. L'errore va su stderr, e
  il comando esce con il codice del primo membro che è fallito.
- Ogni membro usa le sue modalità. `--mode` limita ogni membro a quella
  modalità. Un membro che non la abilita fallisce, e gli altri vanno avanti.
- Ogni membro confronta il valore con il proprio intervallo. Una stessa
  luminosità può essere nell'intervallo per un membro e fuori per un altro.

## Scrivi un target {#write-a-target}

Un target è quello che scrivi dove un comando si aspetta un dispositivo. Ha una
di quattro forme:

| Target | Indica |
| ------ | ------ |
| `AA:BB:CC:DD:EE:FF` | il dispositivo con quell'identità |
| `kitchen` | il dispositivo che la configurazione chiama `kitchen` |
| `ambient` | ogni dispositivo del gruppo `ambient` |
| `H6159` | ogni dispositivo noto di quel modello |

Un comando che controlla una luce, come `on` o `brightness`, accetta
un'identità, un nome o un gruppo. `govee devices` e `govee identify` accettano
qualsiasi numero di target, modello compreso.

Un comando che controlla una luce legge il nome o il gruppo direttamente da
`config.yaml`. `govee devices` elenca ciò che è già noto, quindi fai prima una
scansione: un modello, un nome e un gruppo corrispondono tra i dispositivi
trovati da una scansione.
