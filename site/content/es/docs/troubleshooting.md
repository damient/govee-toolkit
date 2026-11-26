---
title: Solución de problemas
slug: troubleshooting
order: 9
description: Nada responde al escaneo, un comando no hace nada, se rechaza un valor: qué significa cada caso.
source: 3efc01557d239fe5fdbfe28919096d20636a6be3
---

# Solución de problemas

## Nada responde a `govee scan` {#nothing-answers-govee-scan}

Comprueba esto, en orden:

1. **LAN Control está activado.** App Govee Home, tu dispositivo, ajustes,
   **LAN Control**. Todo comando por Wi-Fi necesita ese interruptor.
2. **Una sola red.** Tu ordenador y la luz deben estar en la misma red. La
   Wi-Fi de invitados y algunas redes mesh los separan, incluso en una misma
   casa.
3. **El modelo tiene el interruptor.** No todos los modelos lo tienen. Búscalo
   en [la página de dispositivos]({{base}}devices/), que lista todos los
   modelos a los que Govee da el interruptor.
4. **Tu cortafuegos.** El descubrimiento usa multicast sobre UDP. Un
   cortafuegos que lo bloquea hace invisibles todos los dispositivos.
5. **Un solo programa en el puerto 4002.** Las respuestas vuelven al puerto UDP
   4002, y el sistema entrega cada una a un único programa en ese puerto. Un
   servidor MCP o `govee-dmx` en marcha puede quedárselas. `govee scan` avisa
   cuando ve ese programa: detenlo y vuelve a escanear.

Ejecuta `govee doctor`. Indica lo que puede ver de tu configuración.

## Falta el interruptor LAN Control {#the-lan-control-switch-is-missing}

Comprueba primero que el modelo está en [la página de dispositivos]({{base}}devices/),
que lista todos los modelos a los que Govee da el interruptor. Si está y el
interruptor no aparece:

1. Comprueba que la luz está en la Wi-Fi en la app Govee Home.
2. Desenchufa la luz y vuelve a encenderla.
3. Espera unos 30 minutos y vuelve a mirar los ajustes.

Si el interruptor sigue sin aparecer, contacta con el soporte de Govee.

## Un comando indica éxito y no pasa nada {#a-command-reports-success-and-nothing-happens}

El firmware ignora un comando que no conoce, y no responde nada en ningún caso.
Una sonda fallida y una función no implementada se ven igual desde fuera.

Si el comando viene de `send`, puede que el archivo del dispositivo declare algo
que este modelo no admite. Si viene de un verbo, infórmalo: es un hallazgo
real.

## Se rechaza un valor {#a-value-is-refused}

El toolkit rechaza un valor fuera del rango que declara el archivo del
dispositivo. No lo ajusta. El firmware limitaría el valor en silencio e
indicaría éxito, y creerías que se aplicó un ajuste que no se aplicó.

`govee describe <device>` muestra el rango de cada comando.

## El brillo 0 no apaga la luz {#brightness-0-does-not-turn-the-light-off}

En un modelo verificado, el firmware limita `brightness: 0` a 1. Usa
`govee off` para apagar el dispositivo.

## El brillo por segmento no funciona por Wi-Fi {#per-segment-brightness-does-not-work-over-wi-fi}

El canal de segmentos en `lan` solo transporta color. Ahí el brillo es global.
`ble` transporta el brillo por zona, y `cloud` lo transporta por zona. Es una
propiedad del transporte, no una carencia del código.

## No se encuentra una escena {#a-scene-is-not-found}

`govee scene` lee la biblioteca de escenas del modelo desde una caché en tu
ordenador.

- `scene_list_missing`: ejecuta una vez `govee scenes fetch <modelo>` y vuelve
  a intentarlo.
- `unknown_scene`: `govee scenes list <modelo>` muestra los nombres y los
  códigos.
- `ambiguous_scene`: el nombre coincide con más de una escena. Da un nombre
  completo del error, o un código.

## El escaneo Bluetooth no encuentra nada {#the-bluetooth-scan-finds-nothing}

Un dispositivo Govee deja de anunciarse en cuanto algo se conecta a él. Si la
app de Govee, u otro programa, tiene la conexión, ciérralo y vuelve a escanear.

## Tras una actualización de firmware, el comportamiento cambió {#after-a-firmware-update-behavior-changed}

Las actualizaciones de firmware cambian el comportamiento sin aviso. Un archivo
de dispositivo registra el firmware con el que se midieron sus números. Vuelve
a verificar tras una actualización e informa de lo que cambió.

## Informa de un problema {#report-a-problem}

Abre una issue en [GitHub]({{repo}}/issues) con tu número de modelo, tu
sistema operativo y la salida de `govee doctor`. Oculta el nombre de tu red y
tu clave de API.
