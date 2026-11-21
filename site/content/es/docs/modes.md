---
title: Modos
slug: modes
order: 6
description: Wi-Fi, Bluetooth o la nube: desde dónde llega cada camino a tu luz y qué necesita cada uno.
source: 4f40857299b36f7e9d64f3d83ccb55b68d2271ac
---

# Modos

Un comando viaja hasta tu luz por uno de tres caminos. Tú eliges qué caminos
permites, para cada dispositivo.

| Modo | Velocidad | Llega al dispositivo desde |
| ---- | --------- | -------------------------- |
| {{badge_lan}} | el más rápido | la misma Wi-Fi |
| {{badge_ble}} | rápido | el alcance del Bluetooth |
| {{badge_cloud}} | el más lento | cualquier lugar con internet |

## Qué necesita cada modo {#what-each-mode-needs}

{{badge_lan}} — activa "LAN Control" para el dispositivo en la app Govee Home,
y pon el dispositivo y tu ordenador en la misma red. La Wi-Fi de invitados y
algunas redes mesh los separan.

{{badge_ble}} — un adaptador Bluetooth en tu ordenador, y el dispositivo a su
alcance. Una conexión cada vez: un dispositivo conectado deja de anunciarse,
así que un escaneo mientras otro controlador tiene el enlace no encuentra nada.

{{badge_cloud}} — una clave de API de Govee, y el dispositivo registrado en esa
cuenta. La API tiene límite de peticiones y no admite animación.

## Dónde está la verdad {#where-the-truth-lives}

Lo que alcanza un modo depende del modelo. [La página de dispositivos]({{base}}devices/)
indica lo que responde cada uno.
