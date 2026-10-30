import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { rustEnvironment } from "./rust-env.mjs";
const require = createRequire(import.meta.url);
const result = spawnSync(
  process.execPath,
  [require.resolve("@tauri-apps/cli/tauri.js"), ...process.argv.slice(2)],
  { stdio: "inherit", env: rustEnvironment() },
);
if (result.error) console.error(result.error.message);
process.exit(result.status ?? 1);
