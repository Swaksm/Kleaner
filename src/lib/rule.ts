import { anonymize } from "./report";

const NEW_ISSUE = "https://github.com/Swaksm/Kleaner/issues/new";

/**
 * Adresse d'un formulaire GitHub pré-rempli pour proposer une règle sur un élément inconnu.
 * Rien n'est envoyé par Kleaner : le navigateur s'ouvre, l'utilisateur relit et décide.
 * Le nom d'utilisateur Windows est masqué dans le chemin.
 */
export function ruleProposalUrl(path: string): string {
  const name = path.split("\\").filter(Boolean).pop() ?? path;
  const query = [
    ["template", "rule-proposal.yml"],
    ["title", `[Rule] ${name}`],
    ["path", anonymize(path)],
  ];
  return `${NEW_ISSUE}?${query.map(([k, v]) => `${k}=${encodeURIComponent(v)}`).join("&")}`;
}
