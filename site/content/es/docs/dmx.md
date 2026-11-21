---
title: DMX
slug: dmx
order: 7
description: Controla tus luces desde una consola de iluminación por DMX, igual que un espectáculo controla cualquier otro aparato.
source: 7361202b2432615a85bfa555df6791f68a2134c2
---

# DMX

`govee-dmx` toma el Art-Net que tu consola ya envía y lo escribe en cada luz
por {{badge_lan}}. Tus luces se convierten en aparatos del espectáculo.

## Qué necesitas {#what-you-need}

- Una luz que {{badge_lan}} alcance. [La página de dispositivos]({{base}}devices/)
  indica qué modelos lo hacen.
- Una consola, un servidor de medios o una aplicación de espectáculos que envíe
  Art-Net.
- Un archivo de patch que indique qué luz responde a qué canales.

## Elige una disposición de canales {#pick-a-channel-layout}

Cada luz responde a una de tres disposiciones. Una consola llama a esto una
personalidad.

| Disposición | Canales | Lo que obtienes |
| ----------- | ------- | --------------- |
| `full` | 6 | un color para toda la luz |
| `segment` | 3 + 3 por zona | un color por zona |
| `pixel` | 3 + 3 por LED | un color por LED |

El canal 1 es siempre el dimmer, el canal 2 el modo y el canal 3 la temperatura
de blanco, así un cue pasa de un modelo al siguiente. El blanco cubre toda la
luz, sea cual sea la disposición. Un modelo solo ofrece las disposiciones que
admite su hardware. Cada página de modelo muestra sus tablas, canal a canal:
[el H61A0]({{base}}devices/H61A0/#dmx) es un ejemplo.

## Haz el patch de un aparato {#patch-a-fixture}

Dos comandos preparan la instalación por ti.

```sh
govee-dmx patch      # find the lights and write the patch file
govee-dmx identify   # light one fixture at a time
```

`patch` busca las luces en tu red y da a cada una una dirección de inicio
libre. No toca ninguna dirección que ya hayas fijado, así que puedes volver a
ejecutarlo después de añadir una luz.

`identify` recorre la instalación. Apaga todos los aparatos, después los
enciende uno a uno y nombra la línea del archivo de patch que controla cada
uno. Así etiquetas los aparatos en la consola.

El primer canal es siempre el dimmer. A 0 apaga la luz, así un blackout en la
consola apaga toda la instalación.

## Pon en marcha la instalación {#run-the-rig}

```sh
govee-dmx run        # receive Art-Net and drive the lights
```

`run` escucha Art-Net y escribe cada trama en las luces que nombra el archivo
de patch. Tu consola lista el nodo en cuanto responde a un poll, y ahí haces su
patch como el de cualquier otro nodo. Añade `--scan` para buscar antes las
luces, así un solo comando lleva una instalación de cero a un nodo que la
consola ve.

`--dry-run` muestra cada paquete y lo que cada aparato lee de él, y deja cada
luz como está. Así compruebas un patch antes de que la instalación se mueva.

La ejecución apaga todas las luces al inicio, así la instalación arranca a
oscuras. Cuando una luz se cae de la red, las demás siguen y la ejecución lo
indica.
