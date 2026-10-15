import { describe, expect, it } from "vitest";
import { ruleProposalUrl } from "./rule";

describe("ruleProposalUrl", () => {
  it("masque le nom d'utilisateur et vise le formulaire de règle", () => {
    const url = ruleProposalUrl("C:\\Users\\marie\\AppData\\Local\\Vendor\\App");
    expect(url.startsWith("https://github.com/Swaksm/Kleaner/issues/new?template=rule-proposal.yml")).toBe(true);
    expect(decodeURIComponent(url)).toContain("path=%USERPROFILE%\\AppData\\Local\\Vendor\\App");
    expect(url.toLowerCase()).not.toContain("marie");
  });

  it("met le nom de l'élément dans le titre", () => {
    expect(decodeURIComponent(ruleProposalUrl("D:\\Games\\Foo\\cache.bin"))).toContain("title=[Rule] cache.bin");
  });
});
