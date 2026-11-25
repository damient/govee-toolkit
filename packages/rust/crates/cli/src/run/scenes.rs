//! `scenes fetch` and `scenes list`: the scene library of one model.

use govee_toolkit::exit::{Failure, Writer};
use govee_toolkit::{Scene, scenes};
use serde_json::json;

use crate::cli::Scenes;

pub(super) async fn run(writer: Writer, action: &Scenes) -> Result<(), Failure> {
    let (sku, scenes) = match action {
        Scenes::Fetch { sku } => (sku, scenes::fetch(sku).await.map_err(from)?),
        Scenes::List { sku } => (sku, scenes::load(sku).map_err(from)?),
    };
    match action {
        Scenes::Fetch { .. } => writer.emit(
            &json!({ "sku": sku, "scenes": scenes.len() }),
            &format!("{sku}: {} scenes stored", scenes.len()),
        ),
        Scenes::List { .. } => {
            for scene in &scenes {
                writer.emit(&json!(scene), &line(scene));
            }
        }
    }
    Ok(())
}

fn from(error: scenes::Error) -> Failure {
    Failure::from(govee_toolkit::Error::from(error))
}

/// A scene with no parameters is one the firmware stores.
fn line(scene: &Scene) -> String {
    let kind = if scene.param.is_empty() {
        "stored".to_owned()
    } else {
        format!("type {}, {} bytes", scene.scene_type, scene.param.len())
    };
    format!(
        "{:>5}  {:<24} {:<20} {kind}",
        scene.code, scene.name, scene.category
    )
}
