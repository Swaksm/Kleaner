// Remplace @tauri-apps/plugin-opener dans le mode « demo » : n'ouvre rien.
export const openUrl = (_url: string) => Promise.resolve();
export const revealItemInDir = (_path: string) => Promise.resolve();
