---
title: Noms et groupes
slug: targets
order: 4
description: Donnez un nom à un appareil, réunissez plusieurs appareils dans un groupe, et pilotez-les par ce nom plutôt que par une adresse MAC.
source: 363675951b17dbdc5e606db0fbbadd2c283c3a6f
---

# Noms et groupes

Une adresse MAC est difficile à taper et encore plus à retenir. Donnez un nom à
chaque appareil dans `config.yaml`, réunissez dans un groupe les appareils qui vont
ensemble, et chaque commande accepte ces noms à la place de l'adresse MAC.

## Nommer un appareil {#name-a-device}

`name:` donne un nom à un appareil. `groups:` le place dans un ou plusieurs groupes :

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

Un groupe existe dès qu'un appareil le porte. Un nom et un groupe se comparent en
entier et sans tenir compte de la casse : `Kitchen` trouve `kitchen`, et `kit` ne
trouve rien.

## Piloter un groupe {#drive-a-group}

Une commande envoyée à un groupe part vers tous ses membres à la fois :

<div class="terminal">
<pre><code><span class="prompt">$</span> govee on ambient
<span class="prompt">$</span> govee brightness ambient 40</code></pre>
<button class="copy" type="button" data-copy="govee brightness ambient 40">Copier</button>
</div>

Chaque membre répond pour lui-même :

- Quand un membre échoue, les autres continuent. L'échec part sur stderr, et la
  commande se termine avec le code du premier membre qui a échoué.
- Chaque membre utilise ses propres modes. `--mode` limite chaque membre à ce
  mode. Un membre qui ne l'a pas activé échoue, et les autres continuent.
- Chaque membre compare la valeur à sa propre plage. Une même luminosité peut être
  dans la plage d'un membre et hors de la plage d'un autre.

## Écrire une cible {#write-a-target}

Une cible est ce que vous tapez là où une commande attend un appareil. Elle prend
l'une de quatre formes :

| Cible | Elle désigne |
| ----- | ------------ |
| `AA:BB:CC:DD:EE:FF` | l'appareil qui a cette identité |
| `kitchen` | l'appareil que la configuration nomme `kitchen` |
| `ambient` | tous les appareils du groupe `ambient` |
| `H6159` | tous les appareils connus de ce modèle |

Une commande qui pilote une lumière, comme `on` ou `brightness`, accepte une
identité, un nom ou un groupe. `govee devices` et `govee identify` acceptent un
nombre quelconque de cibles, modèle compris.

Une commande qui pilote une lumière lit le nom ou le groupe directement dans
`config.yaml`. `govee devices` liste ce qui est déjà connu, alors faites d'abord un
scan : un modèle, un nom et un groupe se comparent aux appareils qu'un scan a trouvés.
