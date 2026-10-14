// Vérification de mise à jour, désactivée par défaut : seule exception à « aucun réseau ».
// Lit la liste publique des releases GitHub ; aucune donnée n'est envoyée.

const API = "https://api.github.com/repos/Swaksm/Kleaner/releases?per_page=10";
const KEY = "updateCheck";

export type Update = { version: string; url: string };

/** « 0.1.0-rc7 » → [0, 1, 0, 7] ; une version stable passe après toutes ses préversions. */
function parse(v: string): number[] {
  const [core, pre] = v.replace(/^v/, "").split("-");
  const nums = core.split(".").map((n) => parseInt(n, 10) || 0);
  while (nums.length < 3) nums.push(0);
  return [...nums.slice(0, 3), pre ? parseInt(pre.replace(/\D/g, ""), 10) || 0 : Number.MAX_SAFE_INTEGER];
}

export function isNewer(candidate: string, current: string): boolean {
  const a = parse(candidate);
  const b = parse(current);
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return a[i] > b[i];
  return false;
}

export function updateCheckEnabled(): boolean {
  try {
    return localStorage.getItem(KEY) === "1";
  } catch {
    return false;
  }
}

export function setUpdateCheck(on: boolean) {
  try {
    localStorage.setItem(KEY, on ? "1" : "0");
  } catch {}
}

type Release = { tag_name: string; html_url: string; draft: boolean; prerelease: boolean };

/** Plus récente version publiée que `current` ; les préversions ne sont proposées qu'aux préversions. */
export async function findUpdate(current: string): Promise<Update | null> {
  const res = await fetch(API, { headers: { Accept: "application/vnd.github+json" } });
  if (!res.ok) throw new Error(`GitHub ${res.status}`);
  const releases = (await res.json()) as Release[];
  const onPre = current.includes("-");
  const best = releases
    .filter((r) => !r.draft && (onPre || !r.prerelease) && isNewer(r.tag_name, current))
    .sort((a, b) => (isNewer(a.tag_name, b.tag_name) ? -1 : 1))[0];
  return best ? { version: best.tag_name.replace(/^v/, ""), url: best.html_url } : null;
}
