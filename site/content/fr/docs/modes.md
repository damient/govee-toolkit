---
title: Modes
slug: modes
order: 6
description: Wi-Fi, Bluetooth ou cloud — d'où chaque chemin atteint votre lumière, et ce dont chacun a besoin.
source: 4f40857299b36f7e9d64f3d83ccb55b68d2271ac
---

# Modes

Une commande parvient à votre lumière par l'un de trois chemins. Vous choisissez
les chemins autorisés, appareil par appareil.

| Mode | Vitesse | Il atteint l'appareil depuis |
| ---- | ------- | ---------------------------- |
| {{badge_lan}} | le plus rapide | le même Wi-Fi |
| {{badge_ble}} | rapide | la portée Bluetooth |
| {{badge_cloud}} | le plus lent | partout où il y a internet |

## Ce dont chaque mode a besoin {#what-each-mode-needs}

{{badge_lan}} — activez « LAN Control » pour l'appareil dans l'application Govee Home, et
placez l'appareil et votre ordinateur sur le même réseau. Le Wi-Fi invité et certains
réseaux maillés les séparent.

{{badge_ble}} — un adaptateur Bluetooth sur votre ordinateur, et l'appareil à portée.
Une seule connexion à la fois : un appareil connecté cesse de s'annoncer, donc un scan
lancé pendant qu'un autre contrôleur tient la liaison ne trouve rien.

{{badge_cloud}} — une clé d'API Govee, et l'appareil enregistré sur ce compte.
L'API est limitée en débit, et elle ne transmet aucune animation.

## Où se trouve la vérité {#where-the-truth-lives}

Ce qu'un mode atteint dépend du modèle. [La page des appareils]({{base}}devices/)
indique ce que chacun répond.
