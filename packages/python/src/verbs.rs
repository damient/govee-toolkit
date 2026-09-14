//! The verbs, and what each one needs before it reaches the core.
//!
//! A verb names a `role:`, and the device file names the command. No SKU and
//! no command name is written here.

use govee_toolkit::codec::Args;
use govee_toolkit::{
    DeviceId, DeviceStatus, Error, Govee, Music, Paint, Provisioned, Reply, Resolution,
    SegmentStream, Served, StreamOptions, WifiCredentials,
};

/// Send one device file entry.
pub(crate) async fn send(
    govee: Govee,
    id: DeviceId,
    command: String,
    args: Args,
) -> Result<Served, Error> {
    govee.device(&id).send(&command, &args).await
}

/// Run a command's exchanges and return what its `reply:` layouts captured.
pub(crate) async fn read(
    govee: Govee,
    id: DeviceId,
    command: String,
    args: Args,
) -> Result<Reply, Error> {
    govee.device(&id).read(&command, &args).await
}

/// Ask the device for its state and wait for the answer.
pub(crate) async fn status(govee: Govee, id: DeviceId) -> Result<DeviceStatus, Error> {
    govee.device(&id).status().await
}

/// The mode a command would go over, after a scan if the device is unknown.
pub(crate) async fn ensure_known(govee: Govee, id: DeviceId) -> Result<govee_toolkit::Mode, Error> {
    govee.ensure_known(&id).await
}

/// Turn the device on or off.
pub(crate) async fn power(govee: Govee, id: DeviceId, on: bool) -> Result<Served, Error> {
    govee.device(&id).power(on).await
}

/// Set the level, in the unit the device file declares.
pub(crate) async fn brightness(govee: Govee, id: DeviceId, level: i64) -> Result<Served, Error> {
    govee.device(&id).brightness(level).await
}

/// Set one color.
pub(crate) async fn color(govee: Govee, id: DeviceId, rgb: [u8; 3]) -> Result<Served, Error> {
    govee.device(&id).color(rgb).await
}

/// Set the white temperature.
pub(crate) async fn color_temp(govee: Govee, id: DeviceId, kelvin: i64) -> Result<Served, Error> {
    govee.device(&id).color_temp(kelvin).await
}

/// Play an effect the device renders from its own microphone.
pub(crate) async fn music(govee: Govee, id: DeviceId, music: Music) -> Result<Served, Error> {
    govee.device(&id).music(&music).await
}

/// Paint the segments once.
pub(crate) async fn segment(
    govee: Govee,
    id: DeviceId,
    zones: Option<Vec<u16>>,
    colors: Vec<[u8; 3]>,
    resolution: Resolution,
    gradient: bool,
) -> Result<Served, Error> {
    let paint = Paint {
        zones: zones.as_deref(),
        colors: &colors,
        resolution,
        gradient,
    };
    govee.device(&id).segment(&paint).await
}

/// Ask the firmware to interpolate between zones.
pub(crate) async fn gradient(govee: Govee, id: DeviceId, on: bool) -> Result<Served, Error> {
    govee.device(&id).gradient(on).await
}

/// Put the device on a Wi-Fi network over `ble`.
pub(crate) async fn provision_wifi(
    govee: Govee,
    id: DeviceId,
    credentials: WifiCredentials,
) -> Result<Provisioned, Error> {
    govee.device(&id).provision_wifi(&credentials).await
}

/// Open the raw segment channel.
pub(crate) async fn open_stream(
    govee: Govee,
    id: DeviceId,
    options: StreamOptions,
) -> Result<SegmentStream, Error> {
    govee.device(&id).open_stream(options).await
}

/// What a transfer reported, as one word.
pub(crate) fn provisioned_name(provisioned: Provisioned) -> &'static str {
    match provisioned {
        Provisioned::Accepted => "accepted",
        Provisioned::Sent => "sent",
    }
}
