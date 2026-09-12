import { readFileSync, writeFileSync } from "node:fs";
import { execSync } from "node:child_process";

const version = process.env.npm_package_version;
if (!version) {
	throw new Error("npm_package_version not set — run via `npm version`");
}

// The Rust/Tauri version lives in the workspace root, not src-tauri/Cargo.toml
// — src-tauri inherits it via `version.workspace = true`.
const cargoTomlPath = "Cargo.toml";
const cargoToml = readFileSync(cargoTomlPath, "utf8");
writeFileSync(
	cargoTomlPath,
	cargoToml.replace(/^version = ".*"$/m, `version = "${version}"`),
);

const tauriConfPath = "src-tauri/tauri.conf.json";
const tauriConf = readFileSync(tauriConfPath, "utf8");
writeFileSync(
	tauriConfPath,
	tauriConf.replace(/"version": ".*?"/, `"version": "${version}"`),
);

// Cosmetic only — nothing reads frontend/package.json's version at runtime —
// but kept in sync so it doesn't drift and confuse anyone skimming it.
const frontendPkgPath = "frontend/package.json";
const frontendPkg = JSON.parse(readFileSync(frontendPkgPath, "utf8"));
frontendPkg.version = version;
writeFileSync(frontendPkgPath, JSON.stringify(frontendPkg, null, "\t") + "\n");

// No --offline: on a fresh CI runner this is the very first cargo
// invocation, before Rust/the registry cache are even set up, so an
// offline resolve fails on any dep not already cached.
execSync("cargo update -p legere", { stdio: "inherit" });

execSync(
	"git add Cargo.toml Cargo.lock src-tauri/tauri.conf.json frontend/package.json",
);
