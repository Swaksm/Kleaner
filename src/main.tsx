import React from "react";
import ReactDOM from "react-dom/client";
import "@fontsource/archivo/700.css";
import "@fontsource/archivo/800.css";
import "@fontsource/ibm-plex-sans/400.css";
import "@fontsource/ibm-plex-sans/500.css";
import "@fontsource/ibm-plex-sans/600.css";
import "@fontsource/ibm-plex-mono/400.css";
import "@fontsource/ibm-plex-mono/600.css";
import "./styles/tokens.css";
import "./styles/app.css";
import App from "./App";
import { applyThemeChoice, savedThemeChoice } from "./lib/theme";

// Avant le rendu pour éviter un flash de mauvais thème ; on ne mémorise que les choix explicites.
// Mode « demo » (captures d'écran du README, `npm run demo:build`) : données fictives prêtes avant le rendu.
// Cette branche disparaît de l'exe : MODE vaut « production ».
const ready = import.meta.env.MODE === "demo" ? import("./demo/boot").then((m) => m.boot()) : Promise.resolve();

applyThemeChoice(savedThemeChoice(), false);

ready.then(() =>
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  ),
);
