import { loadConfig } from "./config";
import { createRoot } from "react-dom/client";
import App from "./App";
import { ConfigProvider } from "antd";
import "antd/dist/reset.css";
import "./styles.css";
await loadConfig();
createRoot(document.getElementById("root")!).render(
  <ConfigProvider
    theme={{
      token: {
        colorPrimary: "#0958d9",
        colorInfo: "#0958d9",
        fontSize: 14,
        colorTextDescription: "#595959",
        colorTextTertiary: "#595959",
        borderRadius: 6,
        fontFamily:
          "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif",
      },
    }}
  >
    <App />
  </ConfigProvider>,
);
