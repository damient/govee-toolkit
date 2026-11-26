---
title: Preguntas frecuentes
description: Qué es Govee Toolkit, qué luces Govee alcanza y qué necesitas para controlarlas por Wi-Fi, por Bluetooth o a través de la nube.
source: e290c00ed743d71ab8f736a7316468768037d409
---

## ¿Qué es Govee Toolkit? {#what-is-govee-toolkit} <!-- menu: Qué es -->

Govee Toolkit controla tus luces Govee desde tu propio ordenador. Las enciende
y las apaga, y ajusta el brillo, el color y la temperatura de blanco. En una
tira LED pinta cada zona por separado. Es gratuito y de código abierto, bajo la
licencia MIT.

## ¿Es un producto oficial de Govee? {#is-it-an-official-govee-product} <!-- menu: Oficial o no -->

No. Es un proyecto independiente, sin afiliación con Govee, ni patrocinio ni
respaldo de Govee. El nombre "Govee" identifica las luces con las que funciona.

## ¿Funciona mi luz Govee con él? {#does-my-govee-light-work-with-it} <!-- menu: Luces compatibles -->

[La página de dispositivos]({{base}}devices/) lista todos los modelos que
conoce el toolkit, y lo que responde cada uno por Wi-Fi, por Bluetooth y a
través de la nube. Un modelo que nadie ha sondeado todavía muestra un signo de
interrogación: sin probar, que está muy lejos de roto.

## Mi modelo no está en la lista. ¿Qué puedo hacer? {#my-model-is-not-on-the-list-what-can-i-do} <!-- menu: Un modelo que falta -->

Hay dos formas de añadirlo. Envíanos la luz y la añadimos gratis, o sigue la
guía y añádela tú. [Añadir un dispositivo]({{base}}devices/add/) explica las
dos.

## ¿Necesito saber programar? {#do-i-need-to-know-how-to-code} <!-- menu: Saber programar -->

No. La línea de comandos usa un comando corto por acción, como `govee on` o
`govee color`. [Empieza aquí]({{base}}docs/start/) no necesita nada de código.
Quien programa puede usar el mismo motor desde Rust, Python o Node.js.

## ¿Funciona sin internet? {#does-it-work-without-the-internet} <!-- menu: Sin internet -->

Sí. Por Wi-Fi y por Bluetooth, cada comando va de tu ordenador directamente a
la luz, en tu propia red. La nube es un tercer modo, que solo habilitas si
quieres llegar a tus luces desde cualquier lugar.

## Wi-Fi, Bluetooth o nube: ¿cuál elijo? {#wi-fi-bluetooth-or-cloud-which-one-do-i-pick} <!-- menu: Wi-Fi, Bluetooth o nube -->

- **Wi-Fi** es el más rápido. La luz y tu ordenador comparten una misma red.
- **Bluetooth** llega a una luz a su alcance, directamente desde tu ordenador.
- **Nube** llega a tus luces desde cualquier lugar con acceso a internet.

Tú eliges los modos de cada luz. [Modos]({{base}}docs/modes/) indica lo que
necesita cada uno.

## ¿Qué es LAN Control y cómo lo activo? {#what-is-lan-control-and-how-do-i-turn-it-on} <!-- menu: LAN Control -->

LAN Control es el interruptor que permite que tu propia red llegue a la luz por
Wi-Fi. Abre la app Govee Home, selecciona tu luz, abre sus ajustes y activa
**LAN Control**. Lo haces una vez por luz.
[La página de dispositivos]({{base}}devices/) lista todos los modelos a los que
Govee da el interruptor.

## ¿Necesito una cuenta de Govee o una clave de API? {#do-i-need-a-govee-account-or-an-api-key} <!-- menu: Cuenta y clave de API -->

Solo para el modo nube. Wi-Fi y Bluetooth funcionan en cuanto la luz está
encendida. Para la nube, pide una clave en la app Govee Home: perfil, después
ajustes, después **Apply for API key**.

## ¿Puedo seguir usando la app Govee Home? {#can-i-keep-using-the-govee-home-app} <!-- menu: La app Govee Home -->

Sí. Por Bluetooth, una luz mantiene una sola conexión a la vez: cierra la app
Govee Home cuando el toolkit se conecta por Bluetooth.

## ¿Puedo conectar una luz nueva a mi Wi-Fi sin la app Govee Home? {#can-i-put-a-new-light-on-my-wi-fi-without-the-govee-home-app} <!-- menu: Configurar la Wi-Fi -->

Sí. `govee provision` envía el nombre de la red y la contraseña a la luz por
Bluetooth. Usa una red de 2,4 GHz: las luces Govee no se unen a una de 5 GHz.
[Configurar]({{base}}docs/configure/#wi-fi) da los detalles.

## ¿En qué ordenadores funciona? {#which-computers-does-it-run-on} <!-- menu: Sistemas compatibles -->

Linux, macOS y Windows. El paquete de Python también incluye compilaciones para
Linux en placas ARM, `aarch64` y `armv7`.

## ¿Puedo controlar cada segmento de una tira LED? {#can-i-control-each-segment-of-a-light-strip} <!-- menu: Segmentos -->

Sí, en un modelo que tiene segmentos. Apunta a una zona, a un rango o a toda la
tira en un solo comando. Cada página de modelo indica su número de zonas.

## ¿Pueden mis luces seguir la música o mi pantalla? {#can-my-lights-follow-music-or-my-screen} <!-- menu: Música y pantalla -->

El toolkit envía colores a una tira fotograma tras fotograma, en tiempo real,
por Wi-Fi. Tu propio programa, o uno que elijas, aporta los fotogramas: a
partir de la música, de la pantalla o de un sensor. Un modelo con micrófono
también reproduce sus propios efectos de música: su página de modelo indica los
modos que los transmiten.

## ¿Puedo reproducir las escenas de mi luz? {#can-i-play-the-scenes-of-my-light} <!-- menu: Escenas -->

Sí, en un modelo cuya página indica "Reproducir una escena". `govee scenes fetch
<modelo>` guarda una vez la biblioteca de escenas del modelo, sin cuenta.
`govee scene <dispositivo> "<nombre>"` reproduce después una escena por su
nombre o su código. También puedes guardar un dibujo propio en el dispositivo,
y el dispositivo lo sigue mostrando.

## ¿Puedo controlar mis luces con DMX? {#can-i-control-my-lights-with-dmx} <!-- menu: DMX -->

Sí. El puente DMX, `govee-dmx`, recibe Art-Net de una consola de iluminación,
un servidor de medios o una aplicación de espectáculos, y cada luz se convierte
en un aparato DMX. [DMX]({{base}}docs/dmx/) explica la configuración.

## ¿Puede un asistente de IA controlar mis luces? {#can-an-ai-assistant-control-my-lights} <!-- menu: Asistente de IA -->

Sí. El servidor MCP, `govee-toolkit-mcp`, se ejecuta en tu equipo y da a un
asistente el catálogo de dispositivos y los controles: escaneo, estado,
encendido, brillo, color, segmentos y escenas. [Servidor MCP]({{base}}docs/mcp/) explica
la configuración.

## ¿Funciona con Home Assistant, Homebridge o Matter? {#does-it-work-with-home-assistant-homebridge-or-matter} <!-- menu: Home Assistant, Matter -->

Es lo siguiente en la hoja de ruta: una aplicación de escritorio, y después
Home Assistant, Homebridge y Matter. [La hoja de ruta]({{repo}}/blob/main/docs/roadmap.md)
sigue lo que pide la gente.

## Mi luz no responde. ¿Qué compruebo? {#my-light-does-not-answer-what-do-i-check} <!-- menu: Solución de problemas -->

Comprueba esto primero:

1. LAN Control está activado para la luz.
2. La luz y tu ordenador están en la misma red. La Wi-Fi de invitados los
   separa.
3. Tu cortafuegos deja pasar el multicast sobre UDP.

[Solución de problemas]({{base}}docs/troubleshooting/) cubre el resto.

## ¿Dónde hago una pregunta o informo de un problema? {#where-do-i-ask-a-question-or-report-a-problem} <!-- menu: Contacto -->

Abre una issue en [GitHub]({{repo}}/issues) con tu número de modelo y tu
sistema operativo. Para una luz que quieras enviarnos, escribe a
[contact@gvetk.com](mailto:contact@gvetk.com).
