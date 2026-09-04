# govee-lan-toolkit — Project Brief for Claude Code

## Contexte

SDK non officiel multi-langage pour contrôler les appareils Govee, avec un focus
principal sur des **commandes LAN non documentées** (découvertes par l'auteur du
projet) permettant un contrôle complet (power, brightness, couleur, effets,
scènes, segments) en local, sans passer par le cloud ni le Bluetooth par défaut.
BLE et Cloud API restent disponibles en fallback automatique.

**Priorité absolue : latence minimale sur le chemin LAN.** Le fallback ne doit
jamais ralentir le chemin rapide.

Nom du repo : `govee-lan-toolkit`
Licence : MIT

---

## 1. Structure du repo à générer

```
govee-lan-toolkit/
├── README.md
├── LICENSE
├── CONTRIBUTING.md
├── .github/
│   └── workflows/
│       ├── ci.yml                    # tests sur toutes les PR
│       ├── python-release.yml        # trigger sur packages/python/**
│       ├── node-release.yml          # trigger sur packages/node/**
│       └── php-release.yml           # trigger sur packages/php/**
│
├── docs/
│   └── protocol/
│       ├── lan.md                    # spec LAN officielle + découvertes non doc
│       ├── ble.md                    # notes GATT reverse-engineé par famille de SKU
│       └── cloud-fallback.md         # notes API cloud (auth, rate limits)
│
├── devices/
│   ├── schema.yaml                   # schéma de définition d'un device/SKU
│   └── README.md                     # comment ajouter un nouveau SKU
│
├── packages/
│   ├── python/
│   │   ├── govee_lan_toolkit/
│   │   ├── tests/
│   │   └── pyproject.toml
│   ├── node/
│   │   ├── src/
│   │   ├── test/
│   │   └── package.json
│   ├── php/
│   │   ├── src/
│   │   ├── tests/
│   │   └── composer.json
│   └── artnet-dmx-bridge/
│       ├── src/
│       └── package.json
│
├── apps/
│   ├── playground/
│   │   ├── server/                   # backend Node, réutilise packages/node
│   │   └── web/                      # UI HTML/JS (contrôles + payload brut)
│   └── desktop/
│       ├── main.js                   # wrapper Electron autour du playground
│       └── package.json
│
├── integrations/
│   ├── home-assistant/               # custom_component Python (HACS)
│   └── homebridge/                   # plugin Node (Homebridge)
│
├── tools/
│   └── device-simulator/             # faux device Govee (UDP+BLE) pour tests
│
└── tests/
    └── fixtures/
        ├── lan-captures/             # paquets UDP réels par SKU
        └── ble-captures/             # trames BLE réelles par SKU
```

---

## 2. Cœur du protocole (source de vérité partagée)

- `devices/schema.yaml` définit le schéma commun : SKU, nom de famille, capacités
  supportées (power/brightness/color/colortemp/scenes/segments/sensors), niveau
  de support par transport (LAN complet / LAN partiel / BLE only / cloud only),
  et table de commandes (opcode + structure de payload) par transport.
- Une entrée YAML par SKU ou famille de SKU dans `devices/`.
- Chaque SDK langage (`packages/*`) doit **lire cette DB** plutôt que dupliquer
  la logique protocole — le SDK n'implémente que le transport (socket UDP, BLE,
  HTTP) et le parsing générique.
- `docs/protocol/lan.md` documente : le protocole LAN officiel (multicast
  discovery `239.255.255.250:4001`, réponse `4002`, contrôle `4003`, format
  JSON `{"msg":{"cmd":...,"data":...}}`) **et** les commandes non documentées
  découvertes (à compléter au fur et à mesure — laisser des sections vides
  prêtes à remplir : structure de payload, SKU compatibles, exemples de trames).

---

## 3. Architecture transport : LAN-first avec fallback

Comportement attendu, identique dans chaque langage (Python/Node/PHP) :

1. **Discovery** : scan multicast une seule fois au démarrage + refresh
   périodique en arrière-plan (pas à chaque commande). Cache persistant
   (IP + MAC + capacités) sur disque.
2. **Socket réutilisé** : un socket UDP ouvert par device (ou partagé), jamais
   recréé à chaque envoi.
3. **Fire-and-verify** : la commande LAN part immédiatement sans attente
   bloquante d'ACK ; la vérification d'état se fait en async derrière.
4. **Circuit breaker par device** (pas par appel) :
   - États : `LAN_OK` | `LAN_DEGRADED` | `LAN_DOWN`
   - 2-3 timeouts consécutifs → bascule en `LAN_DEGRADED`, fallback BLE/Cloud
     pour ce device pendant un cooldown (ex. 30s), puis retente LAN.
   - Le choix du transport se base sur l'état déjà connu du breaker, jamais sur
     un nouveau timeout à chaque appel.
5. Ordre de fallback : **LAN → BLE → Cloud**.

Implémenter cette logique une fois clairement en Python et Node (référence),
porter ensuite en PHP.

---

## 4. Packages par langage

- **Python** (`packages/python`) — `pip`, `pyproject.toml`, tests `pytest`.
- **Node.js** (`packages/node`) — `npm`, TypeScript si possible, tests via le
  runner standard du projet.
- **PHP** (`packages/php`) — `composer`, PSR-4, tests via PHPUnit.
- **Bridge Art-Net/DMX** (`packages/artnet-dmx-bridge`) — service qui écoute
  Art-Net, mappe canaux DMX → device + segment Govee, pousse en LAN en
  priorité via `packages/node`. Aucun projet équivalent n'existe dans
  l'écosystème Govee actuellement — c'est un axe différenciant du projet,
  à ne pas négliger.

Chaque package versionné et releasé indépendamment (tags `python-vX.Y.Z`,
`node-vX.Y.Z`, `php-vX.Y.Z`) via GitHub Actions avec path filters.

---

## 5. Playground (interface de test)

Dans `apps/playground/`, pas dans `packages/` (outil de dev, pas une lib
publiée) :

- **Backend** : petit serveur Node (réutilise directement `packages/node`),
  expose une API locale + WebSocket pour push d'état en temps réel.
- **Frontend** : page web simple (pas besoin de framework lourd) :
  - liste des devices détectés avec badge d'état LAN_OK / LAN_DEGRADED / LAN_DOWN
  - par device : toggle power, slider brightness, color picker, dropdown
    effets/scènes (y compris commandes non documentées)
  - log en bas : chaque commande envoyée + timestamp + latence mesurée
  - **champ payload brut** : permet d'envoyer un JSON de commande custom
    directement au device, pour tester une découverte avant de la formaliser
    dans `devices/*.yaml`

---

## 6. App desktop (Electron)

Dans `apps/desktop/` :

- Wrappe le même backend Node + la même UI web du playground dans une fenêtre
  Electron (pas de duplication de code).
- Auto-discovery au lancement (scan multicast automatique, pas de bouton
  manuel).
- Tray icon pour accès rapide.
- Le playground web doit rester utilisable de façon autonome (sans Electron)
  pour du debug rapide.

---

## 7. Intégrations écosystème

Dans `integrations/`, à faire **après** que le cœur du SDK (packages/python
et packages/node) soit stable :

- **Home Assistant** (`integrations/home-assistant/`) : custom_component
  Python distribuable via HACS, consomme `packages/python`. Priorité n°1 —
  audience la plus large, et c'est là que la différenciation LAN
  scènes/segments non documentées a le plus de valeur.
- **Homebridge** (`integrations/homebridge/`) : plugin Node pour HomeKit,
  consomme `packages/node`. Priorité n°2.
- Ne pas implémenter Matter dans l'immédiat — le laisser en note dans le
  README comme piste future une fois le cœur stabilisé.

---

## 8. README (structure à produire)

Le README principal doit inclure, dans cet ordre :

1. Titre + tagline mentionnant explicitement : alternative à l'API officielle
   Govee, contrôle LAN direct avec commandes non documentées, sans
   round-trip cloud.
2. Avertissement clair : projet communautaire non affilié à Govee.
3. Liste de fonctionnalités (LAN direct faible latence, fallback BLE/Cloud,
   SDKs Python/Node/PHP, bridge Art-Net/DMX, playground web, app desktop).
4. Tableau de compatibilité par SKU (lien vers `devices/`).
5. Installation (un exemple par langage).
6. Démarrage rapide (snippet minimal : découverte + allumer une lampe).
7. Architecture (lien vers `docs/protocol/`, schéma LAN→BLE→Cloud).
8. Section Playground / Desktop app.
9. Contribuer (lien `CONTRIBUTING.md`, comment ajouter un SKU/protocole
   découvert).
10. Avertissement légal : reverse engineering réalisé à des fins
    d'interopérabilité, marque "Govee" citée à titre descriptif uniquement,
    aucune affiliation.
11. Licence (MIT).

Inclure aussi une phrase explicite quelque part dans le README, formulée
autour de : *"Alternative to the official Govee API — direct LAN control
with undocumented commands, no cloud round-trip required"* (capte le
SEO du terme "govee api" tout en positionnant clairement la différenciation).

---

## 9. Licence

MIT pour tout le repo (permissif, favorise l'adoption dans des projets tiers
comme Home Assistant/Homebridge). Ne pas utiliser de licence copyleft
(GPL/AGPL).

---

## 10. Ordre de travail suggéré pour Claude Code

1. Scaffolder l'arborescence complète du repo (dossiers + fichiers vides avec
   TODO où pertinent).
2. Écrire `devices/schema.yaml` (schéma) + un exemple de SKU rempli.
3. Écrire `docs/protocol/lan.md` avec la partie officielle documentée
   (discovery, ports, format JSON) et des sections vides prêtes à accueillir
   les découvertes non documentées de l'auteur.
4. Implémenter le transport LAN de référence en Python **et** Node (scan,
   cache, socket réutilisé, circuit breaker, fallback stub).
5. Porter le transport en PHP.
6. Construire le playground (backend + web UI + payload brut).
7. Construire l'app Electron autour du playground.
8. Scaffolder `integrations/home-assistant/` (custom_component minimal,
   turn on/off + brightness en LAN pour commencer).
9. Scaffolder `integrations/homebridge/`.
10. Scaffolder `packages/artnet-dmx-bridge`.
11. Rédiger README, CONTRIBUTING, LICENSE (MIT), CI workflows.

À chaque étape, prioriser la latence et la fiabilité du chemin LAN avant les
fonctionnalités de fallback ou les intégrations tierces.
