import { defineConfig, sessionDrivers } from "astro/config";
import cloudflare from "@astrojs/cloudflare";

export default defineConfig({
  output: "server",
  adapter: cloudflare({
    remoteBindings: false,
  }),
  session: {
    driver: sessionDrivers.null(),
  },
  security: {
    checkOrigin: false,
  },
});
