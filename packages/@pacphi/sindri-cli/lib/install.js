/**
 * @pacphi/sindri-cli postinstall fallback
 *
 * Runs after `npm install @pacphi/sindri-cli`. If the platform-specific optional
 * dependency was installed successfully, this is a no-op. If it was skipped
 * (e.g. cross-platform install, CI environment), we attempt to download the
 * binary directly from GitHub Releases.
 *
 * Set SINDRI_SKIP_DOWNLOAD=1 to disable the download fallback entirely.
 */

"use strict";

const { existsSync, mkdirSync, chmodSync, createWriteStream } = require("node:fs");
const { get } = require("node:https");
const path = require("node:path");

const VERSION = require("../package.json").version;
const REPO = "pacphi/sindri";

const PLATFORM_ASSETS = {
  "darwin-arm64": `sindri-v${VERSION}-aarch64-apple-darwin.tar.gz`,
  "darwin-x64": `sindri-v${VERSION}-x86_64-apple-darwin.tar.gz`,
  "linux-x64": `sindri-v${VERSION}-x86_64-unknown-linux-musl.tar.gz`,
  "linux-arm64": `sindri-v${VERSION}-aarch64-unknown-linux-musl.tar.gz`,
  "win32-x64": `sindri-v${VERSION}-x86_64-pc-windows-msvc.zip`,
};

async function main() {
  if (process.env.SINDRI_SKIP_DOWNLOAD === "1") return;

  const key = `${process.platform}-${process.arch}`;
  const asset = PLATFORM_ASSETS[key];

  if (!asset) {
    // Not a supported platform for download — skip silently
    return;
  }

  // Check if the optional package already installed the binary
  try {
    const { getInstalledBinaryPath } = require("./index.js");
    const bin = getInstalledBinaryPath();
    if (existsSync(bin) && bin !== "sindri") {
      // Binary already present from optional dep
      return;
    }
  } catch {
    // Continue with fallback download
  }

  const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${asset}`;
  const destDir = path.join(__dirname, "..", "bin");
  const destFile = path.join(destDir, process.platform === "win32" ? "sindri.exe" : "sindri");

  if (!existsSync(destDir)) mkdirSync(destDir, { recursive: true });

  process.stdout.write(`@pacphi/sindri-cli: Downloading ${url} …\n`);

  await download(url, destFile);

  if (process.platform !== "win32") {
    chmodSync(destFile, 0o755);
  }

  process.stdout.write(`@pacphi/sindri-cli: Binary installed to ${destFile}\n`);
}

function download(url, dest) {
  return new Promise((resolve, reject) => {
    const file = createWriteStream(dest);
    get(url, (res) => {
      if (res.statusCode === 302 || res.statusCode === 301) {
        file.close();
        return download(res.headers.location, dest).then(resolve).catch(reject);
      }
      if (res.statusCode !== 200) {
        file.close();
        return reject(new Error(`HTTP ${res.statusCode} fetching ${url}`));
      }
      res.pipe(file);
      file.on("finish", () => file.close(resolve));
      file.on("error", reject);
    }).on("error", reject);
  });
}

main().catch((err) => {
  // Postinstall failures must not break the overall install
  process.stderr.write(`@pacphi/sindri-cli: postinstall warning — ${err.message}\n`);
});
