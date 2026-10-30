import { spawnSync } from "node:child_process";
import { rustEnvironment } from "./rust-env.mjs";
const env = rustEnvironment();
const result = spawnSync(env.CARGO, process.argv.slice(2), {
  stdio: "inherit",
  env,
});
if (result.error) console.error(result.error.message);
process.exit(result.status ?? 1);
