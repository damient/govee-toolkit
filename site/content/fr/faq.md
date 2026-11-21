---
title: Questions fréquentes
description: Ce qu'est Govee Toolkit, quelles lumières Govee il atteint, et ce qu'il vous faut pour les piloter en Wi-Fi, en Bluetooth ou par le cloud.
source: c6e77337cfbf1646097cc1a62168315b829f8532
---

## Qu'est-ce que Govee Toolkit ? {#what-is-govee-toolkit} <!-- menu: Ce que c'est -->

Govee Toolkit pilote vos lumières Govee depuis votre propre ordinateur. Il les
allume et les éteint, et il règle la luminosité, la couleur et la température de
blanc. Sur un ruban LED, il peint chaque zone séparément. Il est gratuit et open
source, sous licence MIT.

## Est-ce un produit Govee officiel ? {#is-it-an-official-govee-product} <!-- menu: Officiel ou non -->

Non. C'est un projet indépendant, ni affilié à Govee, ni sponsorisé, ni approuvé par
Govee. Le nom « Govee » désigne les lumières avec lesquelles il fonctionne.

## Ma lumière Govee est-elle compatible ? {#does-my-govee-light-work-with-it} <!-- menu: Lumières compatibles -->

[La page des appareils]({{base}}devices/) recense chaque modèle que le toolkit
connaît, et ce que chacun répond en Wi-Fi, en Bluetooth et par le cloud. Un modèle
que personne n'a encore sondé affiche un point d'interrogation : non testé, ce qui
est très loin de « en panne ».

## Mon modèle n'est pas dans la liste. Que faire ? {#my-model-is-not-on-the-list-what-can-i-do} <!-- menu: Un modèle absent -->

Deux façons de l'y ajouter. Envoyez-nous la lumière et nous l'ajoutons
gratuitement, ou suivez le guide et ajoutez-la vous-même.
[Ajouter un appareil]({{base}}devices/add/) décrit les deux.

## Faut-il savoir programmer ? {#do-i-need-to-know-how-to-code} <!-- menu: Savoir programmer -->

Non. La ligne de commande prend une courte commande par action, comme
`govee on` ou `govee color`. [Commencer ici]({{base}}docs/start/) ne demande aucun
code. Les développeurs peuvent utiliser le même moteur depuis Rust, Python ou
Node.js.

## Fonctionne-t-il sans internet ? {#does-it-work-without-the-internet} <!-- menu: Sans internet -->

Oui. En Wi-Fi et en Bluetooth, chaque commande va directement de votre ordinateur à
la lumière, sur votre propre réseau. Le cloud est un troisième mode, que vous
activez seulement si vous voulez atteindre vos lumières de partout.

## Wi-Fi, Bluetooth ou cloud : lequel choisir ? {#wi-fi-bluetooth-or-cloud-which-one-do-i-pick} <!-- menu: Wi-Fi, Bluetooth ou cloud -->

- **Wi-Fi** est le plus rapide. La lumière et votre ordinateur partagent un même réseau.
- **Bluetooth** atteint une lumière à portée, directement depuis votre ordinateur.
- **Cloud** atteint vos lumières de partout où il y a un accès à internet.

Vous choisissez les modes de chaque lumière. [Modes]({{base}}docs/modes/) indique
ce dont chacun a besoin.

## Qu'est-ce que LAN Control, et comment l'activer ? {#what-is-lan-control-and-how-do-i-turn-it-on} <!-- menu: LAN Control -->

LAN Control est l'interrupteur qui permet à votre propre réseau d'atteindre la
lumière en Wi-Fi. Ouvrez l'application Govee Home, sélectionnez votre lumière,
ouvrez ses réglages et activez **LAN Control**. Vous le faites une fois par lumière.
[La page des appareils]({{base}}devices/) recense chaque modèle auquel Govee donne
l'interrupteur.

## Faut-il un compte Govee ou une clé d'API ? {#do-i-need-a-govee-account-or-an-api-key} <!-- menu: Compte et clé d'API -->

Seulement pour le mode cloud. Le Wi-Fi et le Bluetooth fonctionnent dès que la
lumière est allumée. Pour le cloud, demandez une clé dans l'application Govee Home :
profil, puis réglages, puis **Apply for API key**.

## Puis-je continuer à utiliser l'application Govee Home ? {#can-i-keep-using-the-govee-home-app} <!-- menu: L'application Govee Home -->

Oui. En Bluetooth, une lumière accepte une seule connexion à la fois : fermez
l'application Govee Home quand le toolkit se connecte en Bluetooth.

## Puis-je connecter une nouvelle lumière à mon Wi-Fi sans l'application Govee Home ? {#can-i-put-a-new-light-on-my-wi-fi-without-the-govee-home-app} <!-- menu: Configuration Wi-Fi -->

Oui. `govee provision` envoie le nom du réseau et le mot de passe à la lumière en
Bluetooth. Utilisez un réseau 2,4 GHz : les lumières Govee ne rejoignent pas un
réseau 5 GHz. [Configuration]({{base}}docs/configure/#wi-fi) donne les détails.

## Sur quels ordinateurs fonctionne-t-il ? {#which-computers-does-it-run-on} <!-- menu: Systèmes pris en charge -->

Linux, macOS et Windows. Le paquet Python contient aussi des versions pour Linux
sur cartes ARM, `aarch64` et `armv7`.

## Puis-je piloter chaque segment d'un ruban LED ? {#can-i-control-each-segment-of-a-light-strip} <!-- menu: Segments -->

Oui, sur un modèle qui possède des segments. Adressez une zone, une plage ou le
ruban entier en une seule commande. Chaque page de modèle indique son nombre de
zones.

## Mes lumières peuvent-elles suivre la musique ou mon écran ? {#can-my-lights-follow-music-or-my-screen} <!-- menu: Musique et écran -->

Le toolkit envoie les couleurs à un ruban trame après trame, en temps réel, en
Wi-Fi et en Bluetooth. Votre propre programme, ou un programme de votre choix,
fournit les trames : à partir de la musique, de l'écran ou d'un capteur.

## Puis-je piloter mes lumières en DMX ? {#can-i-control-my-lights-with-dmx} <!-- menu: DMX -->

Oui. Le pont DMX, `govee-dmx`, reçoit l'Art-Net d'une console lumière, d'un serveur
média ou d'un logiciel de spectacle, et chaque lumière devient un projecteur DMX.
[DMX]({{base}}docs/dmx/) explique la mise en place.

## Un assistant IA peut-il piloter mes lumières ? {#can-an-ai-assistant-control-my-lights} <!-- menu: Assistant IA -->

Oui. Le serveur MCP, `govee-toolkit-mcp`, tourne sur votre machine et donne à un
assistant le catalogue des appareils et les commandes : scan, état, alimentation,
luminosité, couleur et segments. [Serveur MCP]({{base}}docs/mcp/) explique la mise
en place.

## Fonctionne-t-il avec Home Assistant, Homebridge ou Matter ? {#does-it-work-with-home-assistant-homebridge-or-matter} <!-- menu: Home Assistant, Matter -->

C'est la suite de la feuille de route : une application de bureau, puis Home
Assistant, Homebridge et Matter. [La feuille de route]({{repo}}/blob/main/docs/roadmap.md)
suit ce que les gens demandent.

## Ma lumière ne répond pas. Que vérifier ? {#my-light-does-not-answer-what-do-i-check} <!-- menu: Dépannage -->

Vérifiez d'abord ces points :

1. LAN Control est activé pour la lumière.
2. La lumière et votre ordinateur sont sur le même réseau. Le Wi-Fi invité les
   sépare.
3. Votre pare-feu laisse passer le multicast en UDP.

[Dépannage]({{base}}docs/troubleshooting/) couvre le reste.

## Où poser une question ou signaler un problème ? {#where-do-i-ask-a-question-or-report-a-problem} <!-- menu: Contact -->

Ouvrez une issue sur [GitHub]({{repo}}/issues) avec la référence de votre modèle et
votre système d'exploitation. Pour une lumière que vous voulez nous envoyer,
écrivez à [contact@gvetk.com](mailto:contact@gvetk.com).
