import { getVersion } from "@tauri-apps/api/app";

/**
 * Version affichée et comparée aux releases : le tag de la release (« v0.1.0-rc14 », injecté par release.yml),
 * sinon celle de tauri.conf.json (toujours « 0.1.0 », qui ferait croire une rc à jour pour toujours).
 */
export function appVersion(): Promise<string> {
  const tag = import.meta.env.VITE_APP_VERSION as string | undefined;
  return tag ? Promise.resolve(tag.replace(/^v/, "")) : getVersion();
}
