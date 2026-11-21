---
title: DMX
slug: dmx
order: 7
description: Pilotez vos lumières depuis une console lumière en DMX, comme un spectacle pilote tous ses autres projecteurs.
source: 7361202b2432615a85bfa555df6791f68a2134c2
---

# DMX

`govee-dmx` prend l'Art-Net que votre console envoie déjà et l'écrit vers chaque
lumière via {{badge_lan}}. Vos lumières deviennent des projecteurs du spectacle.

## Ce qu'il vous faut {#what-you-need}

- Une lumière que {{badge_lan}} atteint. [La page des appareils]({{base}}devices/)
  indique quels modèles le permettent.
- Une console, un serveur média ou un logiciel de spectacle qui envoie de l'Art-Net.
- Un fichier de patch qui indique quelle lumière répond à quels canaux.

## Choisir une disposition de canaux {#pick-a-channel-layout}

Chaque lumière répond à l'une de trois dispositions. Une console appelle cela une
personnalité.

| Disposition | Canaux | Ce que vous obtenez |
| ----------- | ------ | ------------------- |
| `full` | 6 | une couleur pour toute la lumière |
| `segment` | 3 + 3 par zone | une couleur par zone |
| `pixel` | 3 + 3 par LED | une couleur par LED |

Le canal 1 est toujours le gradateur, le canal 2 le mode et le canal 3 la
température de blanc : une mémoire passe ainsi d'un modèle à l'autre. Le blanc
couvre toute la lumière, quelle que soit la disposition. Un modèle ne propose que
les dispositions que son matériel permet. Chaque page de modèle affiche ses tables,
canal par canal — [le H61A0]({{base}}devices/H61A0/#dmx) en est un exemple.

## Patcher un projecteur {#patch-a-fixture}

Deux commandes préparent l'installation pour vous.

```sh
govee-dmx patch      # find the lights and write the patch file
govee-dmx identify   # light one fixture at a time
```

`patch` cherche les lumières sur votre réseau et donne à chacune une adresse de
départ libre. Il ne touche à aucune adresse que vous avez déjà définie : vous
pouvez donc le relancer après avoir ajouté une lumière.

`identify` parcourt l'installation. Il éteint tous les projecteurs, puis les allume
un par un et nomme la ligne du fichier de patch qui pilote chacun. C'est ainsi que
vous nommez les projecteurs sur la console.

Le premier canal est toujours le gradateur. À 0, il éteint la lumière : un
blackout sur la console éteint donc toute l'installation.

## Lancer l'installation {#run-the-rig}

```sh
govee-dmx run        # receive Art-Net and drive the lights
```

`run` écoute l'Art-Net et écrit chaque trame vers les lumières que nomme le fichier
de patch. Votre console liste le nœud dès qu'il répond à un poll, et vous le patchez
là comme n'importe quel autre nœud. Ajoutez `--scan` pour chercher d'abord les
lumières : une seule commande fait ainsi passer une installation de rien à un nœud
que la console voit.

`--dry-run` affiche chaque paquet et ce que chaque projecteur y lit, et laisse
chaque lumière telle quelle. C'est ainsi que vous vérifiez un patch avant que
l'installation ne bouge.

Le lancement éteint toutes les lumières au départ : l'installation démarre donc dans
le noir. Quand une lumière quitte le réseau, les autres continuent et le programme le
signale.
