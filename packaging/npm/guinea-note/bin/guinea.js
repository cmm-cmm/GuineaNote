#!/usr/bin/env node
// Launcher of the guinea-note npm package: runs the native `guinea` binary
// from the platform package npm installed (guinea-note-<platform>-<arch>).
// The binary is the same file as in the GitHub Release archive.
"use strict";
const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const PLATFORMS = {
  "win32-x64": { pkg: "guinea-note-win32-x64", exe: "guinea.exe" },
  "linux-x64": { pkg: "guinea-note-linux-x64", exe: "guinea" },
};
const key = `${process.platform}-${process.arch}`;
const target = PLATFORMS[key];
if (!target) {
  console.error(`guinea-note: no binary for ${key}; releases cover Windows x64 and Linux x64.`);
  console.error("Build from source: https://github.com/cmm-cmm/GuineaNote#build-from-source");
  process.exit(1);
}
let binary;
try {
  binary = path.join(path.dirname(require.resolve(`${target.pkg}/package.json`)), "bin", target.exe);
} catch {
  console.error(`guinea-note: ${target.pkg} is not installed (optional dependencies disabled?).`);
  console.error(`Reinstall with optional dependencies, or: npm install ${target.pkg}`);
  process.exit(1);
}
if (process.platform !== "win32") {
  try {
    // Tarballs packed on Windows carry no executable bit.
    if ((fs.statSync(binary).mode & 0o111) === 0) fs.chmodSync(binary, 0o755);
  } catch {}
}
const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit", windowsHide: false });
if (result.error) {
  console.error(`guinea-note: cannot start ${binary}: ${result.error.message}`);
  process.exit(1);
}
if (result.signal) process.kill(process.pid, result.signal);
process.exit(result.status ?? 1);
