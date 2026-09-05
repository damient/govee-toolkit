# govee

The [govee-toolkit] facade: configuration, mode selection and events.

`lan`, `ble` and `cloud` are modes the user enables per device — never a
fallback chain. One enabled mode means one mode: if the device cannot be
reached, the command fails and says so.

Unofficial and not affiliated with Govee.

[govee-toolkit]: https://github.com/damient/govee-toolkit
