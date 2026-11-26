---
title: Domande frequenti
description: Che cos'è Govee Toolkit, quali luci Govee raggiunge, e cosa ti serve per controllarle via Wi-Fi, via Bluetooth o tramite il cloud.
source: e290c00ed743d71ab8f736a7316468768037d409
---

## Che cos'è Govee Toolkit? {#what-is-govee-toolkit} <!-- menu: Che cos'è -->

Govee Toolkit controlla le tue luci Govee dal tuo computer. Le accende e le
spegne, e imposta la luminosità, il colore e la temperatura del bianco. Su una
striscia LED colora ogni zona separatamente. È libero e open source, con licenza
MIT.

## È un prodotto ufficiale Govee? {#is-it-an-official-govee-product} <!-- menu: Ufficiale o no -->

No. È un progetto indipendente, non affiliato a Govee, né sponsorizzato o
approvato da Govee. Il nome "Govee" identifica le luci con cui funziona.

## La mia luce Govee funziona con Govee Toolkit? {#does-my-govee-light-work-with-it} <!-- menu: Luci compatibili -->

[La pagina dei dispositivi]({{base}}devices/) elenca ogni modello che il toolkit
conosce, e a cosa risponde ognuno via Wi-Fi, via Bluetooth e tramite il cloud.
Un modello che nessuno ha ancora sondato mostra un punto interrogativo: non
testato, che è ben diverso da guasto.

## Il mio modello non è nell'elenco. Cosa posso fare? {#my-model-is-not-on-the-list-what-can-i-do} <!-- menu: Un modello mancante -->

Ci sono due modi per aggiungerlo. Spediscici la luce e la aggiungiamo
gratuitamente, oppure segui la guida e aggiungila tu.
[Aggiungi un dispositivo]({{base}}devices/add/) descrive entrambi.

## Devo saper programmare? {#do-i-need-to-know-how-to-code} <!-- menu: Saper programmare -->

No. La riga di comando richiede un comando breve per ogni azione, come
`govee on` o `govee color`. [Inizia qui]({{base}}docs/start/) non richiede
codice. Gli sviluppatori possono usare lo stesso motore da Rust, Python o
Node.js.

## Funziona senza internet? {#does-it-work-without-the-internet} <!-- menu: Senza internet -->

Sì. Via Wi-Fi e via Bluetooth, ogni comando va dal tuo computer direttamente
alla luce, sulla tua rete. Il cloud è una terza modalità, che abiliti solo se
vuoi raggiungere le tue luci da ovunque.

## Wi-Fi, Bluetooth o cloud: quale scelgo? {#wi-fi-bluetooth-or-cloud-which-one-do-i-pick} <!-- menu: Wi-Fi, Bluetooth o cloud -->

- **Wi-Fi** è la più veloce. La luce e il tuo computer condividono una rete.
- **Bluetooth** raggiunge una luce a portata, direttamente dal tuo computer.
- **Cloud** raggiunge le tue luci da ovunque ci sia accesso a internet.

Scegli tu le modalità per ogni luce. [Modalità]({{base}}docs/modes/) dice cosa
serve a ognuna.

## Che cos'è LAN Control, e come lo attivo? {#what-is-lan-control-and-how-do-i-turn-it-on} <!-- menu: LAN Control -->

LAN Control è l'interruttore che permette alla tua rete di raggiungere la luce
via Wi-Fi. Apri l'app Govee Home, seleziona la tua luce, apri le sue
impostazioni e attiva **LAN Control**. Lo fai una volta per ogni luce.
[La pagina dei dispositivi]({{base}}devices/) elenca ogni modello a cui Govee dà
l'interruttore.

## Mi servono un account Govee o una chiave API? {#do-i-need-a-govee-account-or-an-api-key} <!-- menu: Account e chiave API -->

Solo per la modalità cloud. Wi-Fi e Bluetooth funzionano appena la luce è
accesa. Per il cloud, richiedi una chiave nell'app Govee Home: profilo, poi
impostazioni, poi **Apply for API key**.

## Posso continuare a usare l'app Govee Home? {#can-i-keep-using-the-govee-home-app} <!-- menu: L'app Govee Home -->

Sì. Via Bluetooth, una luce tiene una connessione alla volta: chiudi l'app
Govee Home quando il toolkit si connette via Bluetooth.

## Posso collegare una nuova luce al mio Wi-Fi senza l'app Govee Home? {#can-i-put-a-new-light-on-my-wi-fi-without-the-govee-home-app} <!-- menu: Configurazione Wi-Fi -->

Sì. `govee provision` invia il nome della rete e la password alla luce via
Bluetooth. Usa una rete a 2,4 GHz: le luci Govee non si collegano a una rete a
5 GHz. [Configurazione]({{base}}docs/configure/#wi-fi) dà i dettagli.

## Su quali computer funziona? {#which-computers-does-it-run-on} <!-- menu: Sistemi supportati -->

Linux, macOS e Windows. Il pacchetto Python include anche build per Linux su
schede ARM, `aarch64` e `armv7`.

## Posso controllare ogni segmento di una striscia LED? {#can-i-control-each-segment-of-a-light-strip} <!-- menu: Segmenti -->

Sì, su un modello che ha i segmenti. Indirizza una zona, un intervallo o tutta
la striscia con un solo comando. Ogni pagina di modello indica il suo numero di
zone.

## Le mie luci possono seguire la musica o lo schermo? {#can-my-lights-follow-music-or-my-screen} <!-- menu: Musica e schermo -->

Il toolkit invia colori a una striscia frame dopo frame, in tempo reale, via
Wi-Fi. Il tuo programma, o uno che scegli tu, fornisce i frame: dalla musica,
dallo schermo o da un sensore. Un modello con microfono riproduce anche i suoi
effetti musicali: la sua pagina modello elenca le modalità che li trasportano.

## Posso riprodurre le scene della mia luce? {#can-i-play-the-scenes-of-my-light} <!-- menu: Scene -->

Sì, su un modello la cui pagina elenca "Riprodurre una scena". `govee scenes
fetch <modello>` salva una volta la libreria di scene del modello, senza
account. `govee scene <dispositivo> "<nome>"` riproduce poi una scena con il
suo nome o il suo codice. Puoi anche salvare un tuo disegno sul dispositivo, e
il dispositivo continua a riprodurlo.

## Posso controllare le mie luci con il DMX? {#can-i-control-my-lights-with-dmx} <!-- menu: DMX -->

Sì. Il bridge DMX, `govee-dmx`, riceve Art-Net da una console luci, da un media
server o da un'applicazione per spettacoli, e ogni luce diventa un apparecchio
DMX. [DMX]({{base}}docs/dmx/) spiega la configurazione.

## Un assistente IA può controllare le mie luci? {#can-an-ai-assistant-control-my-lights} <!-- menu: Assistente IA -->

Sì. Il server MCP, `govee-toolkit-mcp`, gira sul tuo computer e dà a un
assistente il catalogo dei dispositivi e i comandi: scansione, stato,
accensione, luminosità, colore, segmenti e scene. [Server MCP]({{base}}docs/mcp/)
spiega la configurazione.

## Funziona con Home Assistant, Homebridge o Matter? {#does-it-work-with-home-assistant-homebridge-or-matter} <!-- menu: Home Assistant, Matter -->

Prossimi nella roadmap: un'applicazione desktop, poi Home Assistant, Homebridge
e Matter. [La roadmap]({{repo}}/blob/main/docs/roadmap.md) segue quello che le
persone chiedono.

## La mia luce non risponde. Cosa controllo? {#my-light-does-not-answer-what-do-i-check} <!-- menu: Risoluzione dei problemi -->

Controlla prima questi punti:

1. LAN Control è attivo per la luce.
2. La luce e il tuo computer sono sulla stessa rete. Il Wi-Fi ospiti li tiene
   separati.
3. Il tuo firewall lascia passare il multicast su UDP.

[Risoluzione dei problemi]({{base}}docs/troubleshooting/) copre il resto.

## Dove faccio una domanda o segnalo un problema? {#where-do-i-ask-a-question-or-report-a-problem} <!-- menu: Contatti -->

Apri una issue su [GitHub]({{repo}}/issues) con il numero del tuo modello e il
tuo sistema operativo. Per una luce che vuoi spedirci, scrivi a
[contact@gvetk.com](mailto:contact@gvetk.com).
