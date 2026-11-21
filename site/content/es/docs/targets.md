---
title: Nombres y grupos
slug: targets
order: 4
description: Da un nombre a un dispositivo, pon varios dispositivos en un grupo y contrólalos por ese nombre en lugar de por una dirección MAC.
source: 363675951b17dbdc5e606db0fbbadd2c283c3a6f
---

# Nombres y grupos

Una dirección MAC es difícil de escribir y más difícil de recordar. Da a cada
dispositivo un nombre en `config.yaml`, pon en un grupo los dispositivos que van
juntos, y todos los comandos aceptan esos nombres en lugar de la dirección MAC.

## Nombra un dispositivo {#name-a-device}

`name:` da un nombre a un dispositivo. `groups:` lo pone en uno o varios
grupos:

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

Un grupo existe en cuanto un dispositivo lo lleva. Un nombre y un grupo
coinciden enteros y sin distinguir mayúsculas, así que `Kitchen` encuentra
`kitchen` y `kit` no encuentra nada.

## Controla un grupo {#drive-a-group}

Un comando enviado a un grupo llega a todos sus miembros a la vez:

<div class="terminal">
<pre><code><span class="prompt">$</span> govee on ambient
<span class="prompt">$</span> govee brightness ambient 40</code></pre>
<button class="copy" type="button" data-copy="govee brightness ambient 40">Copiar</button>
</div>

Cada miembro responde por su cuenta:

- Cuando un miembro falla, los demás siguen. El fallo va a stderr, y el comando
  termina con el código del primer miembro que falló.
- Cada miembro usa sus propios modos. `--mode` restringe todos los miembros a
  ese modo. Un miembro que no lo tiene habilitado falla, y los demás siguen.
- Cada miembro compara el valor con su propio rango. Un mismo brillo puede
  estar dentro del rango de un miembro y fuera del de otro.

## Escribe un destino {#write-a-target}

Un destino es lo que escribes donde un comando espera un dispositivo. Tiene una
de estas cuatro formas:

| Destino | Designa |
| ------- | ------- |
| `AA:BB:CC:DD:EE:FF` | el dispositivo con esa identidad |
| `kitchen` | el dispositivo que la configuración llama `kitchen` |
| `ambient` | todos los dispositivos del grupo `ambient` |
| `H6159` | todos los dispositivos conocidos de ese modelo |

Un comando que controla una luz, como `on` o `brightness`, acepta una
identidad, un nombre o un grupo. `govee devices` y `govee identify` aceptan
cualquier número de destinos, modelos incluidos.

Un comando que controla una luz lee el nombre o el grupo directamente de
`config.yaml`. `govee devices` lista lo que ya se conoce, así que escanea
antes: un modelo, un nombre y un grupo coinciden entre los dispositivos que
encontró un escaneo.
