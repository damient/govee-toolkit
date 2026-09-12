//! The verbs a person types. Each one reaches the device file through a
//! `role:`, so Node and Python get the same verb from the core rather than a
//! second implementation. No command name lives here.

use clap::{Subcommand, ValueEnum};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum Toggle {
    /// Turn the setting on.
    On,
    /// Turn the setting off.
    Off,
}

impl From<Toggle> for bool {
    fn from(toggle: Toggle) -> Self {
        matches!(toggle, Toggle::On)
    }
}

#[derive(Debug, Subcommand)]
pub(crate) enum Verb {
    /// Turn the device on.
    On {
        /// The device identity.
        device: String,
    },

    /// Turn the device off.
    Off {
        /// The device identity.
        device: String,
    },

    /// Set the brightness, in the unit the device file declares.
    Brightness {
        /// The device identity.
        device: String,
        /// The value. Out of range is an error, never a clamp.
        value: i64,
    },

    /// Set one color over the whole device.
    Color {
        /// The device identity.
        device: String,
        /// `#RRGGBB`.
        color: String,
    },

    /// Set the white temperature, in kelvin.
    ///
    /// White and color are mutually exclusive: this ends the color the device
    /// showed.
    Colortemp {
        /// The device identity.
        device: String,
        /// The temperature in kelvin. Out of range is an error, never a clamp.
        kelvin: i64,
    },

    /// Paint addressable zones.
    Segment {
        /// The device identity.
        device: String,
        /// Zone indices, zero-based and comma-separated. Every zone when
        /// absent. A subset needs a mode that paints by zone mask, and takes
        /// one color.
        #[arg(long, value_name = "LIST")]
        zones: Option<String>,
        /// How many zones the frame states: `app`, `native`, or a count. A
        /// count the unit renders as a smaller one is refused.
        #[arg(long, default_value = "app", value_name = "RESOLUTION")]
        resolution: String,
        /// One `#RRGGBB` for every zone, or one per zone, comma-separated.
        /// `-` reads that list from one line of stdin.
        colors: String,
        /// Interpolate between zones, and wrap from the last back to the
        /// first. Refused where the device file can carry the setting
        /// nowhere.
        #[arg(long)]
        gradient: bool,
    },

    /// Set whether the firmware interpolates between zones, without painting.
    ///
    /// The interpolation wraps from the last zone back to the first, so one
    /// lit zone at one end also lights the other. Refused over a mode that
    /// carries the setting inside its painting frame: nothing here holds what
    /// the device shows, so the colors cannot be repainted under the other
    /// setting. Pass `--gradient` to `segment` there, which sets both at once.
    Gradient {
        /// The device identity.
        device: String,
        /// Whether to interpolate.
        #[arg(value_enum)]
        state: Toggle,
    },

    /// Play an effect the device renders from its own microphone.
    ///
    /// The device listens, and nothing streams from here. The effect
    /// identifiers are the mode's own, and `describe` reports the range each
    /// mode takes. Nothing stops the effect: set a color, a temperature or
    /// the power to end it.
    Music {
        /// The device identity.
        device: String,
        /// Which effect. Out of range is an error, never a clamp.
        effect: i64,
        /// How loud the sound must be for the device to answer it. Sent where
        /// the device file declares the argument.
        #[arg(long, default_value_t = 50, value_name = "LEVEL")]
        sensitivity: i64,
        /// Render in fades rather than on the beat.
        #[arg(long)]
        soft: bool,
        /// `#RRGGBB` to impose. The firmware chooses the colors when absent.
        #[arg(long, value_name = "COLOR")]
        color: Option<String>,
    },
}

impl Verb {
    pub(crate) fn device(&self) -> &str {
        match self {
            Self::On { device }
            | Self::Off { device }
            | Self::Brightness { device, .. }
            | Self::Color { device, .. }
            | Self::Colortemp { device, .. }
            | Self::Segment { device, .. }
            | Self::Gradient { device, .. }
            | Self::Music { device, .. } => device,
        }
    }
}
