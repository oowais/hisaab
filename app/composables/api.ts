// Typed wrappers around the Tauri commands in src-tauri/src/lib.rs.

export const isAndroid = typeof navigator !== 'undefined' && /Android/i.test(navigator.userAgent)
