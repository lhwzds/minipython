#!/usr/bin/env node
// SPDX-License-Identifier: MIT

// Downloads the prebuilt mnpy binary for this platform from the GitHub
// release matching this package's version, verifies its SHA-256 against the
// release's checksums.txt, and installs it as bin/mnpy-bin.
//
// MINIPYTHON_SKIP_INSTALL=1 skips the download.
// MINIPYTHON_NPM_ARCHIVE_DIR=<dir> reads the archive and checksums.txt from a
// local directory instead of GitHub (used by the release gate to test the
// package before a release exists); the checksum is still verified.

const https = require("https");
const fs = require("fs");
const path = require("path");
const crypto = require("crypto");
const { execFileSync } = require("child_process");

const VERSION = require("../package.json").version;
const REPO = "lhwzds/minipython";

const PLATFORM_MAP = {
  darwin: {
    x64: "x86_64-apple-darwin",
    arm64: "aarch64-apple-darwin",
  },
  linux: {
    x64: "x86_64-unknown-linux-musl",
    arm64: "aarch64-unknown-linux-musl",
  },
};

function getPlatformTarget(platform = process.platform, arch = process.arch) {
  const targets = PLATFORM_MAP[platform];
  if (!targets) {
    throw new Error(`Unsupported platform: ${platform}`);
  }

  const target = targets[arch];
  if (!target) {
    throw new Error(`Unsupported architecture: ${arch} on ${platform}`);
  }

  return target;
}

function getArchiveName(target) {
  return `minipython-${target}.tar.gz`;
}

function getDownloadUrl(filename) {
  return `https://github.com/${REPO}/releases/download/v${VERSION}/${filename}`;
}

function getChecksumUrl() {
  return `https://github.com/${REPO}/releases/download/v${VERSION}/checksums.txt`;
}

function download(url, redirects = 0) {
  return new Promise((resolve, reject) => {
    if (!url.startsWith("https://")) {
      reject(new Error(`Refusing non-HTTPS download: ${url}`));
      return;
    }
    https
      .get(url, (response) => {
        if ([301, 302, 303, 307, 308].includes(response.statusCode)) {
          response.resume();
          if (redirects >= 5) {
            reject(new Error(`Too many redirects for ${url}`));
            return;
          }
          download(response.headers.location, redirects + 1).then(resolve).catch(reject);
          return;
        }

        if (response.statusCode !== 200) {
          response.resume();
          reject(new Error(`Failed to download ${url}: ${response.statusCode}`));
          return;
        }

        const chunks = [];
        response.on("data", (chunk) => chunks.push(chunk));
        response.on("end", () => resolve(Buffer.concat(chunks)));
        response.on("error", reject);
      })
      .on("error", reject);
  });
}

function computeSha256(buffer) {
  return crypto.createHash("sha256").update(buffer).digest("hex");
}

function parseChecksums(text) {
  const map = new Map();
  for (const line of text.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    const parts = trimmed.split(/\s+/);
    if (parts.length < 2) continue;
    const hash = parts[0];
    const filename = parts[1].replace(/^\*?/, "");
    map.set(filename, hash);
  }
  return map;
}

function verifyChecksum(buffer, filename, checksumText) {
  const expected = parseChecksums(checksumText).get(filename);
  if (!expected) {
    throw new Error(`Checksum not found for ${filename}`);
  }

  const actual = computeSha256(buffer);
  if (actual !== expected) {
    throw new Error(`Checksum mismatch for ${filename}`);
  }
}

async function fetchRelease(archiveName) {
  const localDir = process.env.MINIPYTHON_NPM_ARCHIVE_DIR;
  if (localDir) {
    return {
      archive: fs.readFileSync(path.join(localDir, archiveName)),
      checksums: fs.readFileSync(path.join(localDir, "checksums.txt"), "utf8"),
    };
  }
  const archive = await download(getDownloadUrl(archiveName));
  const checksums = (await download(getChecksumUrl())).toString("utf8");
  return { archive, checksums };
}

function extractTarGz(buffer, destDir) {
  const tmpFile = path.join(destDir, "tmp.tar.gz");
  fs.writeFileSync(tmpFile, buffer);

  try {
    execFileSync("tar", ["-xzf", tmpFile, "-C", destDir], { stdio: "inherit" });
  } finally {
    fs.unlinkSync(tmpFile);
  }
}

async function main() {
  if (process.env.MINIPYTHON_SKIP_INSTALL === "1") {
    return;
  }

  const target = getPlatformTarget();
  const archiveName = getArchiveName(target);
  const binDir = path.join(__dirname, "..", "bin");

  fs.mkdirSync(binDir, { recursive: true });
  const extractDir = fs.mkdtempSync(path.join(binDir, "extract-"));

  try {
    console.log(`Downloading mnpy for ${target}...`);
    const { archive, checksums } = await fetchRelease(archiveName);
    verifyChecksum(archive, archiveName, checksums);

    extractTarGz(archive, extractDir);
    const binaryPath = path.join(binDir, "mnpy-bin");
    fs.copyFileSync(path.join(extractDir, "mnpy"), binaryPath);
    fs.chmodSync(binaryPath, 0o755);

    console.log("mnpy installed successfully.");
  } finally {
    fs.rmSync(extractDir, { recursive: true, force: true });
  }
}

if (require.main === module) {
  main().catch((error) => {
    console.error(`Failed to install mnpy: ${error.message}`);
    process.exit(1);
  });
}

module.exports = {
  computeSha256,
  getArchiveName,
  getDownloadUrl,
  getPlatformTarget,
  parseChecksums,
  verifyChecksum,
};
