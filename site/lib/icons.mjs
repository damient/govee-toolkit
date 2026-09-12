// One icon per capability key, drawn on a 24 box and stroked with
// `currentColor`. They are inline: the site ships no icon font and no
// third-party sprite, and a chip carries its icon wherever it is rendered.
//
// A key the map does not hold gets no icon, so a capability a device file adds
// tomorrow still renders.
// One strip, cut into zones: the outline is the device and the two lines are
// the cuts.
const SEGMENTS =
  '<rect x="2.6" y="5.6" width="18.8" height="12.8" rx="2.6"/>'
  + '<path d="M8.9 5.6v12.8M15.1 5.6v12.8"/>';

const PATHS = {
  power: '<path d="M12 3.2v8.4"/><path d="M6.8 6.8a7.4 7.4 0 1 0 10.4 0"/>',
  brightness:
    '<circle cx="12" cy="12" r="3.6"/><path d="M12 2.6v2.2M12 19.2v2.2M2.6 12h2.2M19.2 12h2.2M5.4 5.4l1.6 1.6M17 17l1.6 1.6M18.6 5.4 17 7M7 17l-1.6 1.6"/>',
  color:
    '<path d="M12 3.2a8.8 8.8 0 1 0 0 17.6 2.1 2.1 0 0 0 1.6-3.5 2.1 2.1 0 0 1 1.6-3.5h1.9a3.7 3.7 0 0 0 3.7-3.7c0-3.8-4-6.9-8.8-6.9Z"/><circle cx="8.2" cy="10.4" r=".9"/><circle cx="12" cy="7.8" r=".9"/><circle cx="15.8" cy="10.8" r=".9"/>',
  colortemp: '<path d="M14 14.6V5.2a2 2 0 1 0-4 0v9.4a4.2 4.2 0 1 0 4 0Z"/><path d="M12 8.6v6.6"/>',
  // The two segment capabilities share one icon: they address the same zones.
  // The color tells them apart.
  segments: SEGMENTS,
  segment_brightness: SEGMENTS,
  music: '<path d="M9 17.4V5.8l11-2v11.6"/><circle cx="6.4" cy="17.4" r="2.6"/><circle cx="17.4" cy="15.4" r="2.6"/>',
};

/** The icon of one capability, or an empty string when the map holds none. */
export function icon(key) {
  const paths = PATHS[key];
  if (!paths) return "";
  return `<svg class="cap-icon" data-cap="${key}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${paths}</svg>`;
}

// One icon per mode, on the same 24 box as the capability icons.
const MODE_PATHS = {
  lan: '<path d="M2.6 8.6a13.4 13.4 0 0 1 18.8 0"/><path d="M6.1 12.4a8.5 8.5 0 0 1 11.8 0"/><path d="M9.6 16.1a3.6 3.6 0 0 1 4.8 0"/><circle cx="12" cy="19.6" r=".9" fill="currentColor" stroke="none"/>',
  ble: '<path d="M8.4 7.6 15.6 16.4 12 19.8V4.2l3.6 3.4L8.4 16.4"/>',
  cloud: '<path d="M7.4 18.6a4.6 4.6 0 0 1-.5-9.2 6 6 0 0 1 11.4 1.6 3.9 3.9 0 0 1-.8 7.6Z"/>',
};

/** The icon of one mode. */
export function modeIcon(mode) {
  const paths = MODE_PATHS[mode];
  if (!paths) return "";
  return `<svg class="mode-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${paths}</svg>`;
}
