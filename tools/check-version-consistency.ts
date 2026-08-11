import { existsSync, readFileSync, readdirSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const failures: string[] = [];

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function readText(path: string): string | undefined {
  try {
    return readFileSync(path, "utf8");
  } catch (error) {
    failures.push(
      `${relative(root, path)}: unable to read (${error instanceof Error ? error.message : String(error)})`,
    );
    return undefined;
  }
}

function readJson(path: string): unknown | undefined {
  const text = readText(path);
  if (text === undefined) return undefined;

  try {
    return JSON.parse(text);
  } catch (error) {
    failures.push(
      `${relative(root, path)}: invalid JSON (${error instanceof Error ? error.message : String(error)})`,
    );
    return undefined;
  }
}

function versionInSection(toml: string, section: string): string | undefined {
  const escapedSection = section.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = toml.match(
    new RegExp(
      `^\\[${escapedSection}\\]\\s*$([\\s\\S]*?)(?=^\\[|(?![\\s\\S]))`,
      "m",
    ),
  );
  if (!match) return undefined;

  const version = match[1].match(/^version\s*=\s*"([^"]*)"\s*(?:#.*)?$/m);
  return version?.[1];
}

function packageSection(toml: string): string | undefined {
  const match = toml.match(/^\[package\]\s*$([\s\S]*?)(?=^\[|(?![\s\S]))/m);
  return match?.[1];
}

function checkWorkspacePackages(cargoToml: string): void {
  const workspace = cargoToml.match(
    /^\[workspace\]\s*$([\s\S]*?)(?=^\[|(?![\s\S]))/m,
  )?.[1];
  const members = workspace?.match(/^members\s*=\s*\[([\s\S]*?)\]/m)?.[1];
  if (!members) {
    failures.push("Cargo.toml: unable to find [workspace].members");
    return;
  }

  const memberPaths = [...members.matchAll(/"([^"]+)"/g)].map(
    (match) => match[1],
  );
  if (memberPaths.length === 0) {
    failures.push("Cargo.toml: [workspace].members contains no paths");
    return;
  }

  for (const memberPath of memberPaths) {
    const manifestPath = join(root, memberPath, "Cargo.toml");
    const manifest = readText(manifestPath);
    if (manifest === undefined) continue;

    const section = packageSection(manifest);
    if (!section) {
      failures.push(
        `${relative(root, manifestPath)}: missing [package] section`,
      );
      continue;
    }
    if (!/^version\.workspace\s*=\s*true\s*(?:#.*)?$/m.test(section)) {
      failures.push(
        `${relative(root, manifestPath)}: [package] must use version.workspace = true`,
      );
    }
  }
}

function workspacePackagePaths(directory: string): string[] {
  const workspaceDirectory = join(root, directory);

  try {
    return readdirSync(workspaceDirectory, { withFileTypes: true })
      .filter((entry) => entry.isDirectory())
      .map((entry) => join(workspaceDirectory, entry.name, "package.json"))
      .filter((path) => existsSync(path))
      .sort();
  } catch (error) {
    failures.push(
      `${directory}: unable to enumerate (${error instanceof Error ? error.message : String(error)})`,
    );
    return [];
  }
}

const rootPackagePath = join(root, "package.json");
const rootPackage = readJson(rootPackagePath);
const rootVersion =
  isRecord(rootPackage) &&
  typeof rootPackage.version === "string" &&
  rootPackage.version.trim()
    ? rootPackage.version
    : undefined;

if (!rootVersion)
  failures.push("package.json: version must be a non-empty string");

const cargoPath = join(root, "Cargo.toml");
const cargoToml = readText(cargoPath);
if (cargoToml) {
  const cargoVersion = versionInSection(cargoToml, "workspace.package");
  if (!cargoVersion) {
    failures.push(
      "Cargo.toml: [workspace.package].version must be a non-empty string",
    );
  } else if (rootVersion && cargoVersion !== rootVersion) {
    failures.push(
      `Cargo.toml: [workspace.package].version (${cargoVersion}) must equal package.json version (${rootVersion})`,
    );
  }
  checkWorkspacePackages(cargoToml);
}

const tauriPath = join(root, "apps", "desktop", "src-tauri", "tauri.conf.json");
const tauriConfig = readJson(tauriPath);
if (!isRecord(tauriConfig) || tauriConfig.version !== "../../../package.json") {
  failures.push(
    "apps/desktop/src-tauri/tauri.conf.json: version must be ../../../package.json",
  );
}

for (const directory of ["apps", "packages"]) {
  for (const path of workspacePackagePaths(directory)) {
    const packageJson = readJson(path);
    if (!isRecord(packageJson) || !("version" in packageJson)) continue;
    if (!rootVersion || packageJson.version !== rootVersion) {
      failures.push(
        `${relative(root, path)}: version (${String(packageJson.version)}) must equal package.json version (${rootVersion ?? "missing"})`,
      );
    }
  }
}

if (failures.length > 0) {
  for (const failure of failures)
    console.error(`version consistency error: ${failure}`);
  process.exit(1);
}

console.log(`Version consistency check passed (${rootVersion}).`);
