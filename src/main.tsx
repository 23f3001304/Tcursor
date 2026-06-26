import React from "react";
import ReactDOM from "react-dom/client";
import "@fontsource-variable/inter";
import { Hud } from "./hud/Hud";
import "./hud/hud.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode><Hud /></React.StrictMode>
);
