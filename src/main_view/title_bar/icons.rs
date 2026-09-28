pub(super) const SOUND_ON_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M11 5 6 9H3v6h3l5 4V5z"/>
  <path d="M15.5 8.5a5 5 0 0 1 0 7"/>
  <path d="M18.5 5.5a9 9 0 0 1 0 13"/>
</svg>
"#;

pub(super) const SOUND_OFF_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M11 5 6 9H3v6h3l5 4V5z"/>
  <path d="m16 9 5 5"/>
  <path d="m21 9-5 5"/>
</svg>
"#;

pub(super) const NOTIFICATIONS_ON_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M18 8a6 6 0 0 0-12 0c0 7-3 7-3 9h18c0-2-3-2-3-9"/>
  <path d="M10 21h4"/>
</svg>
"#;

pub(super) const NOTIFICATIONS_OFF_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M13.73 21a2 2 0 0 1-3.46 0"/>
  <path d="M18.63 13A17.9 17.9 0 0 1 18 8a6 6 0 0 0-9.33-5"/>
  <path d="M6.26 6.26A6 6 0 0 0 6 8c0 7-3 7-3 9h14"/>
  <path d="m2 2 20 20"/>
</svg>
"#;

pub(super) const PNL_VISIBLE_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z"/>
  <circle cx="12" cy="12" r="3"/>
</svg>
"#;

pub(super) const PNL_HIDDEN_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M9.88 9.88A3 3 0 0 0 14.12 14.12"/>
  <path d="M10.73 5.08A10.4 10.4 0 0 1 12 5c6.5 0 10 7 10 7a18.4 18.4 0 0 1-4.14 5.02"/>
  <path d="M6.61 6.61A18.4 18.4 0 0 0 2 12s3.5 7 10 7a10.7 10.7 0 0 0 4.39-.9"/>
  <path d="m2 2 20 20"/>
</svg>
"#;

#[cfg(target_os = "linux")]
pub(super) const MINIMIZE_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round">
  <path d="M5 12h14"/>
</svg>
"#;

#[cfg(target_os = "linux")]
pub(super) const MAXIMIZE_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linejoin="round">
  <rect x="6" y="6" width="12" height="12" rx="1"/>
</svg>
"#;

#[cfg(target_os = "linux")]
pub(super) const CLOSE_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round">
  <path d="M6 6l12 12"/>
  <path d="M18 6 6 18"/>
</svg>
"#;
