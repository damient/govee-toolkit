---
title: Dépannage
slug: troubleshooting
order: 9
description: Rien ne répond au scan, une commande ne fait rien, une valeur est refusée — ce que chaque cas signifie.
source: 3efc01557d239fe5fdbfe28919096d20636a6be3
---

# Dépannage

## Rien ne répond à `govee scan` {#nothing-answers-govee-scan}

Vérifiez ces points, dans l'ordre :

1. **LAN Control est activé.** Application Govee Home, votre appareil, réglages,
   **LAN Control**. Chaque commande en Wi-Fi a besoin de cet interrupteur.
2. **Un seul réseau.** Votre ordinateur et la lumière doivent être sur le même
   réseau. Le Wi-Fi invité et certains réseaux maillés les séparent, même dans une
   seule maison.
3. **Le modèle possède l'interrupteur.** Ce n'est pas le cas de tous. Cherchez-le
   sur [la page des appareils]({{base}}devices/), qui recense chaque modèle auquel
   Govee donne l'interrupteur.
4. **Votre pare-feu.** La découverte utilise le multicast en UDP. Un pare-feu qui le
   bloque rend chaque appareil invisible.
5. **Un seul programme sur le port 4002.** Les réponses reviennent sur le port UDP
   4002, et le système remet chacune à un seul programme sur ce port. Un serveur MCP
   ou `govee-dmx` en cours d'exécution peut les prendre. `govee scan` prévient quand
   il voit ce programme : arrêtez-le et relancez le scan.

Lancez `govee doctor`. Il signale ce qu'il peut voir de votre configuration.

## L'interrupteur LAN Control est absent {#the-lan-control-switch-is-missing}

Vérifiez d'abord que le modèle figure sur [la page des appareils]({{base}}devices/),
qui recense chaque modèle auquel Govee donne l'interrupteur. S'il y figure et que
l'interrupteur n'apparaît pas :

1. Vérifiez que la lumière est en Wi-Fi dans l'application Govee Home.
2. Débranchez la lumière, puis rallumez-la.
3. Attendez environ 30 minutes, puis regardez de nouveau les réglages.

Si l'interrupteur n'apparaît toujours pas, contactez le support Govee.

## Une commande signale un succès et rien ne se passe {#a-command-reports-success-and-nothing-happens}

Le firmware ignore une commande qu'il ne connaît pas, et il ne répond rien dans
les deux cas. Vue de l'extérieur, une sonde qui échoue ressemble à une fonction non
implémentée.

Si la commande vient de `send`, le fichier d'appareil déclare peut-être quelque
chose que ce modèle ne possède pas. Si elle vient d'un verbe, signalez-le — c'est
une vraie découverte.

## Une valeur est refusée {#a-value-is-refused}

Le toolkit refuse une valeur hors de la plage que déclare le fichier d'appareil. Il
ne l'ajuste pas. Le firmware plafonnerait la valeur sans rien dire et signalerait
un succès, et vous croiriez qu'un réglage a été appliqué alors qu'il ne l'a pas été.

`govee describe <device>` affiche la plage de chaque commande.

## La luminosité 0 n'éteint pas la lumière {#brightness-0-does-not-turn-the-light-off}

Sur un modèle vérifié, le firmware ramène `brightness: 0` à 1. Utilisez
`govee off` pour éteindre l'appareil.

## La luminosité par segment ne fonctionne pas en Wi-Fi {#per-segment-brightness-does-not-work-over-wi-fi}

Le canal des segments en `lan` ne transmet que la couleur. La luminosité y est
globale. `ble` transmet la luminosité par zone, et `cloud` aussi. C'est une
propriété du transport, pas un manque dans le code.

## Une scène est introuvable {#a-scene-is-not-found}

`govee scene` lit la bibliothèque de scènes du modèle dans un cache sur votre
ordinateur.

- `scene_list_missing` : lancez une fois `govee scenes fetch <modèle>`, puis
  réessayez.
- `unknown_scene` : `govee scenes list <modèle>` affiche les noms et les codes.
- `ambiguous_scene` : le nom correspond à plusieurs scènes. Donnez un nom
  complet tiré de l'erreur, ou un code.

## Le scan Bluetooth ne trouve rien {#the-bluetooth-scan-finds-nothing}

Un appareil Govee cesse de s'annoncer dès que quelque chose s'y connecte. Si
l'application Govee, ou un autre programme, tient la connexion, fermez-le et
relancez le scan.

## Après une mise à jour du firmware, le comportement a changé {#after-a-firmware-update-behavior-changed}

Les mises à jour du firmware changent le comportement sans prévenir. Un fichier
d'appareil indique le firmware sur lequel ses valeurs ont été mesurées. Vérifiez de
nouveau après une mise à jour, et signalez ce qui a changé.

## Signaler un problème {#report-a-problem}

Ouvrez une issue sur [GitHub]({{repo}}/issues) avec la référence de votre modèle,
votre système d'exploitation et la sortie de `govee doctor`. Masquez le nom de votre
réseau et votre clé d'API.
