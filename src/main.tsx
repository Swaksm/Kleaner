import React from "react";
import ReactDOM from "react-dom/client";
import "@fontsource/ibm-plex-mono/400.css";
import "@fontsource/ibm-plex-mono/600.css";
import "./styles/tokens.css";
import "./styles/app.css";
import App from "./App";
import { micaEnabled } from "./lib/api";
import { applyThemeChoice, savedThemeChoice } from "./lib/theme";

// Avant le rendu pour éviter un flash de mauvais thème ; on ne mémorise que les choix explicites.
// Mode « demo » (captures d'écran du README, `npm run demo:build`) : données fictives prêtes avant le rendu.
// Cette branche disparaît de l'exe : MODE vaut « production ».
const ready = import.meta.env.MODE === "demo" ? import("./demo/boot").then((m) => m.boot()) : Promise.resolve();

applyThemeChoice(savedThemeChoice(), false);
// Windows 11 : barre latérale translucide sur l'effet Mica (voir lib.rs).
micaEnabled().then((on) => on && document.documentElement.classList.add("mica"), () => {});

ready.then(() =>
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  ),
);
