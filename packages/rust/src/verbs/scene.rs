//! Play a scene of the scene library.
//!
//! A scene with parameters goes out on the entry marked `role: scene`. A
//! scene without them is one the firmware stores, and goes out on the entry
//! marked `role: scene_builtin`. A device file that marks neither fails the
//! call: nothing is approximated.

use crate::codec::{ArgRole, Args, Role};
use crate::device::DeviceHandle;
use crate::error::Result;
use crate::event::Served;
use crate::scenes::{self, Scene};

impl DeviceHandle<'_> {
    /// Play `scene`. Serves [`Role::Scene`], or [`Role::SceneBuiltin`] for a
    /// scene with no parameters.
    ///
    /// The device answers nothing on `lan`: a scene it does not play looks
    /// like one it plays.
    ///
    /// # Errors
    ///
    /// As for [`DeviceHandle::power`], for the role above and its arguments.
    /// A scene type outside the range of the argument marked
    /// `role: scene_type` is an error: the transfer would carry another
    /// format.
    pub async fn scene(&self, scene: &Scene) -> Result<Served> {
        let code = i64::from(scene.code);
        if scene.param.is_empty() {
            let entry = self.role_entry(Role::SceneBuiltin)?;
            let args = Args::new().int(entry.arg(ArgRole::SceneCode)?, code);
            return entry.send(&args).await;
        }
        let entry = self.role_entry(Role::Scene)?;
        let args = Args::new()
            .int(entry.arg(ArgRole::SceneCode)?, code)
            .int(entry.arg(ArgRole::SceneType)?, scene.scene_type)
            .bytes(entry.arg(ArgRole::SceneParam)?, scene.param.clone());
        entry.send(&args).await
    }

    /// Play the scene that `query` names in the cached library of the
    /// device's model: a code, a full name, or the name that the variants of
    /// one scene share. Reads the cache, and sends no request to the cloud.
    ///
    /// # Errors
    ///
    /// [`crate::scenes::Error`] if the library is not cached or `query`
    /// matches no scene or more than one, then as for [`DeviceHandle::scene`].
    pub async fn scene_named(&self, query: &str) -> Result<Served> {
        let call = self.resolve()?;
        let sku = call.sku();
        let library = scenes::load(sku)?;
        let scene = scenes::find(sku, &library, query)?;
        self.scene(scene).await
    }
}
