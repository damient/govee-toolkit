use govee_toolkit::codec::Mode;
use govee_toolkit::{DeviceId, DeviceStatus, Govee};
use serde_json::{Value, json};

use crate::output::{Failure, Writer};

pub(super) async fn run(govee: &Govee, writer: &Writer, id: &DeviceId) -> Result<(), Failure> {
    let handle = govee.device(id);
    let call = handle.resolve()?;
    let mode = call.mode();
    let status = call.status().await?;
    writer.emit(&as_json(&status, mode), &as_text(&status, mode));
    Ok(())
}

// The core renders the fields, so this line and the `status` event line agree.
fn as_json(status: &DeviceStatus, mode: Mode) -> Value {
    let mut record = status.to_json();
    if let Some(fields) = record.as_object_mut() {
        fields.insert("mode".to_owned(), json!(mode.to_string()));
    }
    record
}

fn as_text(status: &DeviceStatus, mode: Mode) -> String {
    format!("{}  {mode}  {status}", status.id)
}
