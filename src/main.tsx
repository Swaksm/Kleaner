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
applyThemeChoice(savedThemeChoice(), false);

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
