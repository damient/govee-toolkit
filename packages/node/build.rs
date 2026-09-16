//! What napi needs at build time: the linker arguments for the Node binary,
//! and the type information the `.d.ts` is generated from.

fn main() {
    napi_build::setup();
}
