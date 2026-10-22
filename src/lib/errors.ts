// Les erreurs de Rust sont en anglais (elles vont aussi au journal) : les plus courantes sont traduites ici.
import { translate, type Key } from "../i18n/useT";

const KNOWN: [RegExp, Key][] = [
  [/was not in the last list/, "error.staleList"],
  [/is the last copy left/, "error.lastCopy"],
  [/\((protected location|system file)\)/, "error.protected"],
  [/\(link or junction on the path\)/, "error.link"],
  [/\(path not found\)/, "error.notFound"],
  [/Turn off "Start with Windows" first/, "error.autostartFirst"],
  [/admin(istrator)? rights/i, "error.admin"],
];

export function friendlyError(message: string): string {
  const hit = KNOWN.find(([re]) => re.test(message));
  return hit ? translate(hit[1]) : message;
}
