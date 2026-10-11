// Diagnose Linux headless vs virtual-display Firefox without mocking graphics.
/* global document */
import { firefox } from "@playwright/test";
for (const headless of [true, false]) {
  const browser = await firefox.launch({ headless });
  try {
    const page = await browser.newPage();
    const graphics = await page.evaluate(() => {
      const canvas = document.createElement("canvas");
      const gl = canvas.getContext("webgl2");
      return gl
        ? {
            webgl2: true,
            version: gl.getParameter(gl.VERSION),
            renderer: gl.getParameter(gl.RENDERER),
          }
        : { webgl2: false };
    });
    console.log(JSON.stringify({ browser: "firefox", headless, ...graphics }));
    if (!headless && !graphics.webgl2) {
      throw new Error(
        "Firefox WebGL2 unavailable under the CI virtual display",
      );
    }
  } finally {
    await browser.close();
  }
}
