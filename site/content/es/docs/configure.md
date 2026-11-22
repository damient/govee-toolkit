---
title: Configurar
slug: configure
order: 3
description: El config.yaml que nombra tus dispositivos, el archivo .env que lee el toolkit, las credenciales Wi-Fi que un dispositivo necesita para unirse a tu red y la clave de API que necesita el modo cloud.
source: ac7a5ca370cd6102f6679a62f9e74a3d695e661d
---

# Configurar

El toolkit lee dos archivos. `config.yaml` nombra tus dispositivos e indica qué
modo usa cada uno. `.env` contiene las credenciales.

`lan` y `ble` envían los comandos tal cual. Lee esta página cuando conectes un
dispositivo a tu red Wi-Fi, o cuando habilites `cloud`.

## El archivo `config.yaml` {#the-config-yaml-file}

Mientras no escribas este archivo, todos los dispositivos usan `lan`. Escríbelo
tú en la carpeta de configuración de tu sistema, o indica otra ruta con
`GOVEE_CONFIG` o `--config`.

| Sistema | Carpeta de configuración |
| --- | --- |
| macOS | `~/Library/Application Support/govee-toolkit` |
| Linux | `$XDG_CONFIG_HOME/govee-toolkit`, si no `~/.config/govee-toolkit` |
| Windows | `%APPDATA%\govee-toolkit` |

Cada dispositivo se identifica por la dirección MAC que comunica. `govee scan`
las lista:

<div class="terminal">
<pre><code><span class="prompt">$</span> govee scan
AA:BB:CC:DD:EE:FF  H6159  -  [lan]
11:22:33:44:55:66  H619A  -  [lan]</code></pre>
<button class="copy" type="button" data-copy="govee scan">Copiar</button>
</div>

```yaml
defaults:
  modes: [lan]

devices:
  "AA:BB:CC:DD:EE:FF":
    name: "kitchen"
  "11:22:33:44:55:66":
    modes: [lan, ble]
```

`modes:` lista los modos que puede usar un dispositivo, por orden de
preferencia. La [página de modos]({{base}}docs/modes/) describe cada modo, y
[nombres y grupos]({{base}}docs/targets/) describe `name:` y `groups:`.

El toolkit rechaza una clave que no conoce, así que una errata falla al
arrancar. `govee doctor` indica qué está mal en el archivo.

## El archivo `.env` {#the-env-file}

Una variable que exportas dura lo que dura la shell. Un archivo `.env` sigue
ahí en una shell nueva. El toolkit lee el primer archivo que encuentra, en este
orden:

1. El archivo que indica `--env-file`, o el que indica `GOVEE_ENV_FILE`. Si ese
   archivo no existe, es un error, porque alguien pidió ese archivo.
2. `.env` en el directorio en el que trabajas, y después en cada directorio
   superior. La búsqueda se detiene tras el directorio que contiene `.git`, tras
   tu directorio personal, o en la raíz del sistema de archivos.
3. `.env` en la carpeta de configuración.

`credentials.env`, en la carpeta de configuración, va después y aporta los
nombres que ningún otro archivo define. Studio escribe ahí la clave de API
cuando importas una copia de seguridad.

El toolkit solo lee los nombres `GOVEE_*` de un `.env`, así que un mismo
archivo sirve para varios proyectos. Una variable que ya está en tu entorno
tiene prioridad sobre el archivo, y `govee --no-env` omite el archivo.

`govee doctor` indica de qué archivo vienen las variables:

<div class="terminal">
<pre><code><span class="prompt">$</span> govee doctor
variables from `/home/you/lights/.env`
no problem found</code></pre>
<button class="copy" type="button" data-copy="govee doctor">Copiar</button>
</div>

## Wi-Fi {#wi-fi}

{{badge_lan}} — un dispositivo responde por `lan` en cuanto está en tu propia
red Wi-Fi. La app Govee Home lo conecta a ella, y `govee provision` hace lo
mismo por `ble`. Dos variables aportan el nombre de la red y la contraseña, así
la contraseña no queda en el historial de tu shell:

```bash
GOVEE_WIFI_SSID=my-network
GOVEE_WIFI_PASSWORD=the password
```

<div class="terminal">
<pre><code><span class="prompt">$</span> govee provision AA:BB:CC:DD:EE:FF</code></pre>
<button class="copy" type="button" data-copy="govee provision AA:BB:CC:DD:EE:FF">Copiar</button>
</div>

`--ssid` y `--password` aceptan los mismos valores en la línea de comandos, y
`--open` se une a una red abierta. El comando necesita una compilación con
Bluetooth: `cargo install govee-toolkit-cli --features ble`.

Cuatro cosas que debes saber antes de ejecutarlo:

- Usa una red de 2,4 GHz. Ningún dispositivo Govee se une a una de 5 GHz.
- La contraseña viaja en texto plano, sin intercambio de claves. Cualquier
  equipo al alcance del Bluetooth mientras se ejecuta el comando puede leerla.
- El dispositivo acusa recibo de la transferencia, y el informe dice si aceptó
  las credenciales. No dice si el dispositivo se unió a la red: compruébalo
  después en la red.
- Cierra antes la app Govee Home. Ocupa la única conexión que acepta la radio.

## La clave de API {#the-api-key}

{{badge_cloud}} — este modo pasa por la propia API de Govee y necesita una
clave. [Empieza aquí]({{base}}docs/start/) explica cómo pedirla en la app.
Pásala al toolkit como `GOVEE_API_KEY`:

```bash
GOVEE_API_KEY=7c41b9d2-5a6e-4f08-9b31-2d84ec55af10
```

No pongas nunca la clave en `config.yaml`. Ese archivo acaba en los informes de
errores. El toolkit nunca registra la clave en los logs y nunca la escribe en la
caché de dispositivos.
