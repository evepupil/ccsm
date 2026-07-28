import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ToastProvider } from "@heroui/react";

import App from "./App";
import { useSystemTheme } from "./lib/theme";
import "./styles.css";

const root = document.getElementById("root");
if (!root) {
  throw new Error("Missing #root element");
}

function Root() {
  useSystemTheme();
  return (
    <>
      <App />
      <ToastProvider placement="bottom end" />
    </>
  );
}

createRoot(root).render(
  <StrictMode>
    <Root />
  </StrictMode>,
);
