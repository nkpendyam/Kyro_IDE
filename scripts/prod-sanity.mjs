import process from "node:process";
import path from "node:path";
import { existsSync, readFileSync } from "node:fs";

const strictMode = process.argv.includes("--strict") || process.env.CI === "true";
const root = process.cwd();

const errors = [];
const warnings = [];

function fail(message) {
  errors.push(message);
}

function warn(message) {
  warnings.push(message);
}

function readJson(relPath) {
  const fullPath = path.join(root, relPath);
  if (!existsSync(fullPath)) {
    fail(`Missing file: ${relPath}`);
    return null;
  }

  try {
    return JSON.parse(readFileSync(fullPath, "utf-8"));
  } catch (error) {
    fail(`Invalid JSON at ${relPath}: ${error.message}`);
    return null;
  }
}

function validateTauriConfig() {
  const config = readJson("src-tauri/tauri.conf.json");
  if (!config) return;

  const frontendDist = config?.build?.frontendDist;
  if (frontendDist !== "../out") {
    fail(`src-tauri/tauri.conf.json build.frontendDist must be "../out" (found: ${frontendDist ?? "undefined"})`);
  }
}

function validateNextConfig() {
  const nextConfigPath = path.join(root, "next.config.ts");
  if (!existsSync(nextConfigPath)) {
    fail("Missing file: next.config.ts");
    return;
  }

  const content = readFileSync(nextConfigPath, "utf-8");
  if (!content.includes('output: "export"')) {
    fail("next.config.ts must set output: \"export\" for Tauri static frontend builds");
  }

  if (content.includes("ignoreBuildErrors: true")) {
    fail("next.config.ts should not hardcode typescript.ignoreBuildErrors: true");
  }
}

function validateVersion() {
  const versionPath = path.join(root, "VERSION");
  if (!existsSync(versionPath)) {
    warn("VERSION file not found");
    return;
  }

  const version = readFileSync(versionPath, "utf-8").trim();
  if (!/^\d+\.\d+\.\d+([-.][\w.]+)?$/.test(version)) {
    fail(`VERSION must be semantic-version-like (found: ${version})`);
  }
}

function validateFeatureEnvVar(flagVar, urlVar) {
  const enabled = process.env[flagVar] === "1" || process.env[flagVar] === "true";
  if (!enabled) return;

  if (!process.env[urlVar]) {
    const message = `${flagVar} is enabled but ${urlVar} is not set`;
    if (strictMode) {
      fail(message);
    } else {
      warn(message);
    }
  }
}

function validateFeatureGates() {
  validateFeatureEnvVar("KYRO_ENABLE_AIRLLM", "KYRO_AIRLLM_URL");
  validateFeatureEnvVar("KYRO_ENABLE_OLLAMA", "KYRO_OLLAMA_URL");
  validateFeatureEnvVar("KYRO_ENABLE_PICOCLAW", "KYRO_PICOCLAW_URL");
  validateFeatureEnvVar("KYRO_ENABLE_N8N", "KYRO_N8N_URL");
}

validateTauriConfig();
validateNextConfig();
validateVersion();
validateFeatureGates();

if (warnings.length > 0) {
  console.warn("Production sanity warnings:");
  for (const warning of warnings) {
    console.warn(`- ${warning}`);
  }
}

if (errors.length > 0) {
  console.error("Production sanity check failed:");
  for (const error of errors) {
    console.error(`- ${error}`);
  }
  process.exit(1);
}

console.log("Production sanity check passed.");
