import { readFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const repoRoot = resolve(dirname(scriptPath), "..");
const htmlPath = join(repoRoot, "apps", "desktop", "out", "index.html");

let html: string;

try {
  html = readFileSync(htmlPath, "utf8");
} catch (error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  console.error(`Unable to read ${relative(repoRoot, htmlPath)}: ${message}`);
  process.exit(1);
}

const headEndIndex = html.indexOf("</head>");
const bodyIndex = html.indexOf("<body");

if (headEndIndex === -1 || bodyIndex === -1) {
  console.error("Unable to locate </head> and <body in index.html.");
  process.exit(1);
}

const emotionStyleTagPattern = /<style\b[^>]*\bdata-emotion(?:\s|=|>|\/)/g;
const emotionStyleTagStarts = Array.from(
  html.matchAll(emotionStyleTagPattern),
  (match: RegExpMatchArray) => match.index,
);

if (emotionStyleTagStarts.length === 0) {
  console.error("No Emotion style tags were found in index.html.");
  process.exit(1);
}

if (
  emotionStyleTagStarts.some(
    (index: number | undefined) =>
      index === undefined || index >= headEndIndex || index >= bodyIndex,
  )
) {
  console.error("Emotion style tags must appear before </head> and <body.");
  process.exit(1);
}

console.log(
  `UI style placement check passed (${emotionStyleTagStarts.length} Emotion style tags in <head>).`,
);
