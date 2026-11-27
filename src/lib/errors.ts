// Les erreurs de Rust sont en anglais (elles vont aussi au journal) : les plus courantes sont traduites ici.
import { translate, type Key } from "../i18n/useT";

const KNOWN: [RegExp, Key][] = [
  [/was not in the last list/, "error.staleList"],
  [/is the last copy left/, "error.lastCopy"],
  [/\((protected location|system file)\)/, "error.protected"],
  [/\(link or junction on the path\)/, "error.link"],
  [/\(path not found\)/, "error.notFound"],
  [/Turn off "Start with Windows" first/, "error.autostartFirst"],
  [/the selection on .* too big for the Recycle Bin/, "error.binTotal"],
  [/would be deleted permanently/, "error.noBin"],
  [/account is not an administrator/, "error.standardAccount"],
  [/admin(istrator)? rights/i, "error.admin"],
  [/\(outside allowed locations\)/, "error.outside"],
  [/\(online-only file\)/, "error.cloudOnly"],
  [/\(a program is installed there\)/, "error.programInside"],
  [/\(part of a code project\)/, "error.codeProject"],
  [/is in use, close the programs/, "error.inUse"],
  [/not enough free space/, "error.noSpace"],
  [/^Refused: .* already exists$/, "error.exists"],
  [/^Cancelled$/, "error.cancelled"],
  [/^The two folders differ, nothing was changed/, "error.foldersDiffer"],
];

export function friendlyError(message: string): string {
  const hit = KNOWN.find(([re]) => re.test(message));
  return hit ? translate(hit[1]) : message;
}
