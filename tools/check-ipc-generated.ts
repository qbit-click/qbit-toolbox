import { spawnSync } from "node:child_process";
import { readFileSync, rmSync } from "node:fs";

const generated = "packages/ipc/generated/index.ts";
const temporary = `${generated}.${process.pid}.tmp`;

function printCapturedOutput(stdout: string, stderr: string): void {
  if (stdout) process.stdout.write(stdout);
  if (stderr) process.stderr.write(stderr);
}

try {
  const generation = spawnSync(
    "cargo",
    [
      "+1.97.1",
      "run",
      "-p",
      "ipc-contracts",
      "--bin",
      "generate-ipc",
      "--",
      temporary,
    ],
    { encoding: "utf8" },
  );
  if (generation.error || generation.status !== 0) {
    console.error(
      `IPC binding generation failed: ${generation.error?.message ?? `cargo exited with status ${generation.status ?? "unknown"}`}`,
    );
    printCapturedOutput(generation.stdout ?? "", generation.stderr ?? "");
    process.exitCode = generation.status ?? 1;
  } else {
    const normalizeText = (text: string) => text.replace(/\r\n?/g, "\n");
    if (
      normalizeText(readFileSync(temporary, "utf8")) !==
      normalizeText(readFileSync(generated, "utf8"))
    ) {
      console.error("IPC bindings are stale. Run: bun run ipc:generate");
      process.exitCode = 1;
    }
  }
} finally {
  rmSync(temporary, { force: true });
}
