---
title: Configurazione
slug: configure
order: 3
description: Il config.yaml che dà un nome ai tuoi dispositivi, il file .env che il toolkit legge, le credenziali Wi-Fi che servono a un dispositivo per entrare nella tua rete, e la chiave API che serve alla modalità cloud.
source: 31e56e8665d485c372e3101fcdd80473e879657c
---

# Configurazione

Il toolkit legge due file. `config.yaml` dà un nome ai tuoi dispositivi e dice
quale modalità usa ognuno. `.env` contiene le credenziali.

`lan` e `ble` inviano i comandi così come sono. Leggi questa pagina quando
colleghi un dispositivo alla tua rete Wi-Fi, o quando abiliti `cloud`.

## Il file `config.yaml` {#the-config-yaml-file}

Finché non scrivi questo file, ogni dispositivo usa `lan`. Scrivilo tu in
`~/.config/govee-toolkit/config.yaml`, oppure indica un altro percorso con
`GOVEE_CONFIG` o `--config`.

Ogni dispositivo ha come chiave l'indirizzo MAC che comunica. `govee scan` li
elenca:

<div class="terminal">
<pre><code><span class="prompt">$</span> govee scan
AA:BB:CC:DD:EE:FF  H6159  -  [lan]
11:22:33:44:55:66  H619A  -  [lan]</code></pre>
<button class="copy" type="button" data-copy="govee scan">Copia</button>
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

`modes:` elenca le modalità che un dispositivo può usare, dalla prima scelta in
giù. La [pagina delle modalità]({{base}}docs/modes/) descrive ogni modalità, e
[Nomi e gruppi]({{base}}docs/targets/) descrive `name:` e `groups:`.

Il toolkit rifiuta una chiave che non conosce, quindi un errore di battitura
fallisce all'avvio. `govee doctor` segnala cosa c'è di sbagliato nel file.

## Il file `.env` {#the-env-file}

Una variabile che esporti dura quanto la shell. Un file `.env` sopravvive a una
nuova shell. Il toolkit legge il primo file che trova, in quest'ordine:

1. Il file indicato da `--env-file`, o il file indicato da `GOVEE_ENV_FILE`. Un
   file assente qui è un errore, perché qualcuno ha chiesto proprio quel file.
2. `.env` nella directory in cui lavori, poi in ogni directory sopra di essa.
   La ricerca si ferma dopo la directory che contiene `.git`, dopo la tua home
   directory, o alla radice del file system.
3. `~/.config/govee-toolkit/.env`.

Il toolkit legge solo i nomi `GOVEE_*` di un `.env`, quindi un solo file può
servire più progetti. Una variabile già presente nel tuo ambiente vince sul
file, e `govee --no-env` salta il file.

`govee doctor` dice da quale file vengono le variabili:

<div class="terminal">
<pre><code><span class="prompt">$</span> govee doctor
variables from `/home/you/.config/govee-toolkit/.env`
no problem found</code></pre>
<button class="copy" type="button" data-copy="govee doctor">Copia</button>
</div>

## Wi-Fi {#wi-fi}

{{badge_lan}} — un dispositivo risponde via `lan` quando è sulla tua rete Wi-Fi.
L'app Govee Home ce lo mette, e `govee provision` fa lo stesso via `ble`. Due
variabili forniscono il nome della rete e la password, così la password resta
fuori dalla cronologia della shell:

```bash
GOVEE_WIFI_SSID=my-network
GOVEE_WIFI_PASSWORD=the password
```

<div class="terminal">
<pre><code><span class="prompt">$</span> govee provision AA:BB:CC:DD:EE:FF</code></pre>
<button class="copy" type="button" data-copy="govee provision AA:BB:CC:DD:EE:FF">Copia</button>
</div>

`--ssid` e `--password` accettano gli stessi valori da riga di comando, e
`--open` si collega a una rete aperta. Il comando richiede una build che include
il Bluetooth: `cargo install govee-toolkit-cli --features ble`.

Quattro cose da sapere prima di eseguirlo:

- Usa una rete a 2,4 GHz. Nessun dispositivo Govee si collega a una rete a 5 GHz.
- La password viaggia in chiaro, senza scambio di chiavi. Chiunque sia a
  portata Bluetooth mentre il comando è in esecuzione la può leggere.
- Il dispositivo conferma il trasferimento, e il resoconto dice se ha accettato
  le credenziali. Non dice se il dispositivo è entrato nella rete: controlla la
  rete dopo.
- Chiudi prima l'app Govee Home. Tiene l'unica connessione che la radio accetta.

## La chiave API {#the-api-key}

{{badge_cloud}} — questa modalità passa dall'API di Govee e richiede una chiave.
[Inizia qui]({{base}}docs/start/) spiega come richiederla nell'app. Passala al
toolkit come `GOVEE_API_KEY`:

```bash
GOVEE_API_KEY=7c41b9d2-5a6e-4f08-9b31-2d84ec55af10
```

Non mettere mai la chiave in `config.yaml`. Quel file finisce nelle
segnalazioni di bug. Il toolkit non registra mai la chiave nei log e non la
scrive mai nella cache dei dispositivi.
