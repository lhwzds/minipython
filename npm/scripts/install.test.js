#!/usr/bin/env node
// SPDX-License-Identifier: MIT

const assert = require("assert");
const {
  computeSha256,
  getArchiveName,
  getDownloadUrl,
  getPlatformTarget,
  parseChecksums,
  verifyChecksum,
} = require("./install");
const { version } = require("../package.json");

assert.strictEqual(
  computeSha256(Buffer.from("abc", "utf8")),
  "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
);

assert.strictEqual(getPlatformTarget("darwin", "arm64"), "aarch64-apple-darwin");
assert.strictEqual(getPlatformTarget("darwin", "x64"), "x86_64-apple-darwin");
assert.strictEqual(getPlatformTarget("linux", "x64"), "x86_64-unknown-linux-musl");
assert.strictEqual(getPlatformTarget("linux", "arm64"), "aarch64-unknown-linux-musl");
assert.throws(() => getPlatformTarget("win32", "x64"), /Unsupported platform: win32/);
assert.throws(() => getPlatformTarget("linux", "ia32"), /Unsupported architecture: ia32/);
assert.strictEqual(getArchiveName("x86_64-apple-darwin"), "minipython-x86_64-apple-darwin.tar.gz");
assert.strictEqual(
  getDownloadUrl("minipython-x86_64-unknown-linux-musl.tar.gz"),
  `https://github.com/lhwzds/minipython/releases/download/v${version}/minipython-x86_64-unknown-linux-musl.tar.gz`,
);

const checksums = parseChecksums(`
ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  minipython-x86_64-apple-darwin.tar.gz
e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 *minipython-aarch64-apple-darwin.tar.gz
`);

assert.strictEqual(
  checksums.get("minipython-x86_64-apple-darwin.tar.gz"),
  "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
);
assert.strictEqual(
  checksums.get("minipython-aarch64-apple-darwin.tar.gz"),
  "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
);

const listing = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  minipython-x86_64-apple-darwin.tar.gz\n";
verifyChecksum(Buffer.from("abc", "utf8"), "minipython-x86_64-apple-darwin.tar.gz", listing);
assert.throws(
  () => verifyChecksum(Buffer.from("abd", "utf8"), "minipython-x86_64-apple-darwin.tar.gz", listing),
  /Checksum mismatch/,
);
assert.throws(
  () => verifyChecksum(Buffer.from("abc", "utf8"), "minipython-aarch64-apple-darwin.tar.gz", listing),
  /Checksum not found/,
);

console.log("install.js tests passed");
