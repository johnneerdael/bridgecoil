// Resolve the actual rustup compiler so a Homebrew rustc cannot shadow mobile targets.
import { spawnSync } from "node:child_process";
import { dirname, delimiter } from "node:path";
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const env = { ...process.env };
for (const [tool, key] of [
  ["rustc", "RUSTC"],
  ["rustdoc", "RUSTDOC"],
  ["cargo", "CARGO"],
]) {
  const result = spawnSync("rustup", ["which", tool], { encoding: "utf8" });
  if (result.status !== 0) {
    process.stderr.write(
      result.stderr || "Install a Rust toolchain with rustup.\n",
    );
    process.exit(result.status ?? 1);
  }
  env[key] = result.stdout.trim();
}
env.PATH = dirname(env.RUSTC) + delimiter + (env.PATH ?? "");
const result = spawnSync(
  process.execPath,
  [require.resolve("@tauri-apps/cli/tauri.js"), ...process.argv.slice(2)],
  { stdio: "inherit", env },
);
if (result.error) console.error(result.error.message);
process.exit(result.status ?? 1);
