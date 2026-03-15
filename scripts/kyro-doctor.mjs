import os from "node:os";
import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import process from "node:process";

const args = new Set(process.argv.slice(2));
const checkOptionals = args.has("--optionals") || process.env.KYRO_DOCTOR_OPTIONALS === "1";

const results = [];

function run(command, commandArgs = ["--version"], options = {}) {
  return spawnSync(command, commandArgs, {
    encoding: "utf-8",
    shell: false,
    ...options,
  });
}

function addResult(name, status, details, required = true) {
  results.push({ name, status, details, required });
}

function checkCommand(name, command, commandArgs = ["--version"], required = true) {
  const result = run(command, commandArgs);
  if (result.error || result.status !== 0) {
    addResult(name, required ? "missing" : "optional-missing", result.error?.message ?? "Not found in PATH", required);
    return false;
  }

  const output = `${result.stdout || ""}${result.stderr || ""}`.trim().split("\n")[0] ?? "available";
  addResult(name, "ok", output, required);
  return true;
}

function checkWebView2() {
  if (process.platform !== "win32") {
    addResult("WebView2 Runtime", "skipped", "Windows-only prerequisite", true);
    return;
  }

  const regPath = "HKLM\\SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
  const result = run("reg", ["query", regPath]);
  if (result.status === 0) {
    addResult("WebView2 Runtime", "ok", "Registry key detected", true);
    return;
  }

  addResult("WebView2 Runtime", "missing", "Install via: winget install Microsoft.EdgeWebView2Runtime", true);
}

function checkTauriProjectPrereqs() {
  const packagePath = path.join(process.cwd(), "package.json");
  if (!existsSync(packagePath)) {
    addResult("Tauri CLI dependency", "missing", "package.json not found in current directory", true);
    return;
  }

  try {
    const content = JSON.parse(readFileSync(packagePath, "utf-8"));
    const hasTauriCli = Boolean(content.devDependencies?.["@tauri-apps/cli"]);
    if (hasTauriCli) {
      addResult("Tauri CLI dependency", "ok", "@tauri-apps/cli declared in devDependencies", true);
    } else {
      addResult("Tauri CLI dependency", "missing", "Add @tauri-apps/cli to devDependencies", true);
    }
  } catch (error) {
    addResult("Tauri CLI dependency", "missing", `Unable to parse package.json: ${error.message}`, true);
  }
}

function checkAirLlm() {
  const url = process.env.KYRO_AIRLLM_URL ?? "http://127.0.0.1:8765/health";
  const js = `
    const url = ${JSON.stringify(url)};
    fetch(url)
      .then((r) => {
        if (!r.ok) {
          console.error('HTTP ' + r.status);
          process.exit(2);
        }
        process.exit(0);
      })
      .catch((e) => {
        console.error(e.message);
        process.exit(3);
      });
  `;
  const result = run(process.execPath, ["-e", js]);
  if (result.status === 0) {
    addResult("AirLLM service", "ok", `Reachable at ${url}`, false);
  } else {
    addResult("AirLLM service", "optional-missing", `Unavailable at ${url}`, false);
  }
}

function checkOptionalsGroup() {
  checkAirLlm();
  checkCommand("Ollama", "ollama", ["--version"], false);
  checkCommand("PicoClaw", "picoclaw", ["--version"], false);
  checkCommand("n8n", "n8n", ["--version"], false);
}

console.log("Kyro Doctor Report");
console.log(`Platform: ${os.platform()} ${os.release()} (${os.arch()})`);
console.log("");

checkCommand("Node.js", "node", ["--version"], true);
checkCommand("Bun", "bun", ["--version"], true);
checkCommand("Rustc", "rustc", ["--version"], true);
checkCommand("Cargo", "cargo", ["--version"], true);
checkTauriProjectPrereqs();
checkWebView2();

if (checkOptionals) {
  checkOptionalsGroup();
} else {
  addResult("Optional integrations", "skipped", "Run `bun run doctor:full` to check AirLLM/Ollama/PicoClaw/n8n", false);
}

const requiredFailures = results.filter((item) => item.required && item.status === "missing");
const optionalFailures = results.filter((item) => !item.required && item.status === "optional-missing");

for (const result of results) {
  let icon = "✓";
  if (result.status === "missing") icon = "✗";
  if (result.status === "optional-missing") icon = "!";
  if (result.status === "skipped") icon = "-";

  const requirement = result.required ? "required" : "optional";
  console.log(`${icon} [${requirement}] ${result.name}: ${result.details}`);
}

console.log("");
console.log(`Summary: ${requiredFailures.length} required missing, ${optionalFailures.length} optional unavailable`);

if (requiredFailures.length > 0) {
  process.exitCode = 1;
}
