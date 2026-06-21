import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vitest/config";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  test: {
    environment: "happy-dom",
    globals: false,
    include: ["src/**/*.{test,spec}.ts"],
    setupFiles: ["./vitest.setup.ts"],
    // Run test files serially in a single worker. Under Node 26 the
    // multi-worker pool leaves happy-dom workers spinning instead of exiting,
    // which hangs the run; one worker exits cleanly and the suite is small
    // enough that serial execution costs nothing.
    pool: "threads",
    fileParallelism: false,
    maxWorkers: 1,
  },
});
