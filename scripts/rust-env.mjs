import { spawnSync } from "node:child_process";
import { dirname, delimiter } from "node:path";
export function rustEnvironment() {
  const env = { ...process.env };
  for (const [tool, key] of [
    ["rustc", "RUSTC"],
    ["rustdoc", "RUSTDOC"],
    ["cargo", "CARGO"],
  ]) {
    const result = spawnSync("rustup", ["which", tool], { encoding: "utf8" });
    if (result.status !== 0)
      throw new Error(result.stderr || "Install a Rust toolchain with rustup.");
    env[key] = result.stdout.trim();
  }
  env.PATH = dirname(env.RUSTC) + delimiter + (env.PATH ?? "");
  return env;
}
