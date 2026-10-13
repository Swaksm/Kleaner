import { expect, test } from "vitest";
import de from "./de.json";
import en from "./en.json";
import es from "./es.json";
import fr from "./fr.json";
import it from "./it.json";
import pt from "./pt.json";

const all = { en, fr, es, de, it, pt };

test("toutes les langues ont exactement les mêmes clés que l'anglais", () => {
  const keys = Object.keys(en).sort();
  for (const [lang, dict] of Object.entries(all)) expect(Object.keys(dict).sort(), lang).toEqual(keys);
});

test("aucune traduction vide", () => {
  for (const [lang, dict] of Object.entries(all)) {
    for (const [key, value] of Object.entries(dict)) expect(value.trim(), `${lang} ${key}`).not.toBe("");
  }
});

test("aucun caractère de contrôle (\\r, \\n… issus d'un mauvais échappement)", () => {
  for (const [lang, dict] of Object.entries(all)) {
    for (const [key, value] of Object.entries(dict)) expect(/[\u0000-\u001f]/.test(value), `${lang} ${key}`).toBe(false);
  }
});

test("les chemins Windows gardent leurs « \\ »", () => {
  for (const [lang, dict] of Object.entries(all)) {
    expect(dict["report.saved"], lang).toContain("%LOCALAPPDATA%\\Kleaner\\rapports");
  }
});
