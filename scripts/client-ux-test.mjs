import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const baseUrl = process.env.BASE_URL || "http://127.0.0.1:8000";
const outDir = join(root, ".tmp", "client-ux");
const scenarios = [
  { name: "desktop", width: 1280, height: 720, mobile: false },
  { name: "mobile", width: 390, height: 844, mobile: true }
];

class Cdp {
  constructor(ws) {
    this.ws = ws;
    this.nextId = 1;
    this.pending = new Map();
    this.listeners = new Map();
    ws.addEventListener("message", (event) => {
      const data = typeof event.data === "string" ? event.data : Buffer.from(event.data).toString("utf8");
      const message = JSON.parse(data);
      if (message.id && this.pending.has(message.id)) {
        const { resolve: ok, reject: fail } = this.pending.get(message.id);
        this.pending.delete(message.id);
        if (message.error) fail(new Error(message.error.message));
        else ok(message.result || {});
        return;
      }
      if (message.method && this.listeners.has(message.method)) {
        for (const listener of this.listeners.get(message.method)) listener(message.params || {});
      }
    });
  }

  static async connect(url) {
    const ws = new WebSocket(url);
    await new Promise((resolve, reject) => {
      ws.addEventListener("open", resolve, { once: true });
      ws.addEventListener("error", reject, { once: true });
    });
    return new Cdp(ws);
  }

  send(method, params = {}) {
    const id = this.nextId++;
    this.ws.send(JSON.stringify({ id, method, params }));
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
    });
  }

  once(method) {
    return new Promise((resolve) => {
      const listener = (params) => {
        const listeners = this.listeners.get(method) || [];
        this.listeners.set(method, listeners.filter((item) => item !== listener));
        resolve(params);
      };
      const listeners = this.listeners.get(method) || [];
      listeners.push(listener);
      this.listeners.set(method, listeners);
    });
  }

  close() {
    this.ws.close();
  }
}

await mkdir(outDir, { recursive: true });

let staticServer;
let browser;
let cdp;

try {
  if (!(await canFetch(baseUrl))) {
    staticServer = spawn(process.platform === "win32" ? "python" : "python3", [
      "-m",
      "http.server",
      "8000",
      "--bind",
      "127.0.0.1"
    ], { cwd: root, stdio: "ignore" });
    await waitForHttp(baseUrl, 12_000);
  }

  const chromePath = findChrome();
  const debugPort = await freePort();
  const profileDir = join(root, ".tmp", `chrome-profile-${process.pid}`);
  await rm(profileDir, { recursive: true, force: true });

  let browserLog = "";
  browser = spawn(chromePath, [
    "--headless=new",
    "--disable-gpu",
    "--disable-dev-shm-usage",
    "--disable-breakpad",
    "--disable-crash-reporter",
    "--no-first-run",
    "--no-default-browser-check",
    "--remote-allow-origins=*",
    `--remote-debugging-port=${debugPort}`,
    `--user-data-dir=${profileDir}`,
    "about:blank"
  ], { stdio: ["ignore", "ignore", "pipe"] });
  browser.stderr.on("data", (chunk) => {
    browserLog += chunk.toString();
  });

  await waitForBrowserEndpoint(`http://127.0.0.1:${debugPort}/json/version`, browser, () => browserLog, 12_000);
  const targets = await json(`http://127.0.0.1:${debugPort}/json/list`);
  const pageTarget = targets.find((target) => target.type === "page") || targets[0];
  cdp = await Cdp.connect(pageTarget.webSocketDebuggerUrl);
  await cdp.send("Page.enable");
  await cdp.send("Runtime.enable");

  const results = [];
  for (const scenario of scenarios) {
    await cdp.send("Emulation.setDeviceMetricsOverride", {
      width: scenario.width,
      height: scenario.height,
      deviceScaleFactor: 1,
      mobile: scenario.mobile
    });
    await navigate(`${baseUrl}/?geek=1`);
    const result = await evaluateClientTest(scenario);
    const screenshot = await cdp.send("Page.captureScreenshot", { format: "png", fromSurface: true });
    const screenshotPath = join(outDir, `${scenario.name}.png`);
    await writeFile(screenshotPath, Buffer.from(screenshot.data, "base64"));
    results.push({ ...result, screenshot: screenshotPath });
  }

  const failures = results.flatMap((result) => result.failures.map((failure) => `${result.scenario}: ${failure}`));
  console.log(JSON.stringify({ baseUrl, screenshots: results.map((result) => result.screenshot), results }, null, 2));
  if (failures.length) {
    console.error(`Client UX test failed:\n${failures.map((failure) => `- ${failure}`).join("\n")}`);
    process.exitCode = 1;
  }
} finally {
  if (cdp) cdp.close();
  if (browser) browser.kill();
  if (staticServer) staticServer.kill();
}

async function navigate(url) {
  const loaded = cdp.once("Page.loadEventFired");
  await cdp.send("Page.navigate", { url });
  await loaded;
}

async function evaluateClientTest(scenario) {
  const expression = `(${clientTestSource()})(${JSON.stringify(scenario)})`;
  const response = await cdp.send("Runtime.evaluate", {
    expression,
    awaitPromise: true,
    returnByValue: true,
    timeout: 30_000
  });
  if (response.exceptionDetails) {
    throw new Error(response.exceptionDetails.text || "Runtime.evaluate failed");
  }
  return response.result.value;
}

function clientTestSource() {
  return async function runClientUxTest(scenario) {
    const failures = [];
    const notes = [];
    const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const assert = (condition, message) => {
      if (!condition) failures.push(message);
    };
    const visible = (element) => {
      if (!element) return false;
      const style = getComputedStyle(element);
      return style.display !== "none" && style.visibility !== "hidden" && element.getClientRects().length > 0;
    };
    const text = (selector) => document.querySelector(selector)?.textContent.trim() || "";
    const waitFor = async (predicate, message, timeout = 8000) => {
      const started = performance.now();
      while (performance.now() - started < timeout) {
        if (predicate()) return;
        await delay(30);
      }
      failures.push(message);
    };

    await window.geekTools.open();
    await waitFor(() => document.querySelector("#geek-tools")?.open, "modal did not open");

    const api = await import("/wasm/geek.js");
    const tools = api.tools;
    const groups = api.toolGroups;
    const modal = document.querySelector("#geek-tools");
    const workbench = document.querySelector("[data-tool-workbench]");
    const localTool = document.querySelector("[data-local-tool]");
    const textPanel = document.querySelector("[data-tool-text-panel]");
    const inputField = document.querySelector("[data-tool-input-field]");
    const input = document.querySelector("[data-tool-input]");
    const output = document.querySelector("[data-tool-output]");
    const optionWrap = document.querySelector("[data-tool-option-wrap]");
    const optionLabel = document.querySelector("[data-tool-option-label]");
    const option = document.querySelector("[data-tool-option]");
    const optionHelp = document.querySelector("[data-tool-option-help]");
    const explain = document.querySelector("[data-tool-explain]");
    const run = document.querySelector("[data-tool-run]");
    const groupPanel = document.querySelector("[data-group-panel]");
    const groupToolField = document.querySelector("[data-group-tool-field]");
    const groupTool = document.querySelector("[data-group-tool]");
    const groupInputField = document.querySelector("[data-group-input-field]");
    const groupInput = document.querySelector("[data-group-input]");
    const groupOptionField = document.querySelector("[data-group-option-field]");
    const groupOptionLabel = document.querySelector("[data-group-option-label]");
    const groupOption = document.querySelector("[data-group-option]");
    const groupOptionHelp = document.querySelector("[data-group-option-help]");
    const groupOutput = document.querySelector("[data-group-output]");
    const imagePanel = document.querySelector("[data-image-panel]");
    const imageFile = document.querySelector("[data-image-file]");
    const imageWidth = document.querySelector("[data-image-width]");
    const imageHeight = document.querySelector("[data-image-height]");
    const imageFormat = document.querySelector("[data-image-format]");
    const imageQuality = document.querySelector("[data-image-quality]");
    const imageCanvas = document.querySelector("[data-image-canvas]");
    const imageOutput = document.querySelector("[data-image-output]");
    const imageInfo = document.querySelector("[data-image-info]");
    const imageDownload = document.querySelector("[data-image-download]");
    const passPanel = document.querySelector("[data-password-panel]");
    const passOutput = document.querySelector("[data-pass-output]");
    const qrPanel = document.querySelector("[data-qr-panel]");
    const qrOutput = document.querySelector("[data-qr-output]");
    const qrInfo = document.querySelector("[data-qr-info]");
    const qrDownload = document.querySelector("[data-qr-download]");
    const qrCanvas = document.querySelector("[data-qr-canvas]");
    const phonePanel = document.querySelector("[data-phone-panel]");
    const phoneInput = document.querySelector("[data-phone-input]");
    const phoneResults = document.querySelector("[data-phone-results]");
    const phoneOutput = document.querySelector("[data-phone-output]");
    const seloPanel = document.querySelector("[data-selo-panel]");
    const seloKind = document.querySelector("[data-selo-kind]");
    const seloAction = document.querySelector("[data-selo-action]");
    const seloInputField = document.querySelector("[data-selo-input-field]");
    const seloInput = document.querySelector("[data-selo-input]");
    const seloResults = document.querySelector("[data-selo-results]");
    const seloOutput = document.querySelector("[data-selo-output]");

    assert(tools.length >= 40, `expected at least 40 tools, found ${tools.length}`);
    assert(groups.length >= 8, `expected grouped tool UI, found ${groups.length} groups`);
    assert(text("[data-tool-count]") === `${groups.length} groups / ${tools.length} tools`, "tool count should describe grouped local tools");
    assert(tools.length === 45, `expected Page audit removed and Random added, found ${tools.length} tools`);
    assert(groups.length === 10, `expected Page audit removed and Random added, found ${groups.length} groups`);
    assert(!api.findTool("security-checklist"), "security checklist tool should be removed");
    assert(api.findTool("random-bytes"), "random bytes tool should be present");
    assert(groups.some((group) => group.id === "random-values"), "random group should be present");
    assert(!groups.some((group) => group.id === "page-audit"), "page audit group should be removed");
    assert(!document.querySelector("[data-audit-source]"), "page audit source selector should be removed");
    assert(!document.querySelector("[data-tool-grid]"), "external tool cards should be removed");
    assert(!document.querySelector("[data-tool-filter]"), "external tool filter should be removed");
    assert(!document.body.textContent.includes("MD5Decrypt"), "old external cards are still visible");

    const modalRect = modal.getBoundingClientRect();
    assert(modalRect.width <= window.innerWidth - 24, "modal overflows viewport horizontally");
    if (!scenario.mobile) {
      assert(modalRect.width <= 1125, "desktop modal is wider than intended");
      assert(modalRect.width <= 985, "desktop modal is wider than adjusted target");
      assert(modalRect.width >= 640, "desktop modal is too narrow");
    }
    assert(document.documentElement.scrollWidth <= window.innerWidth + 2, "page has horizontal overflow");
    assert(getComputedStyle(workbench).overflowY !== "visible", "workbench should own modal scrolling");

    const outputFor = (group) => {
      if (group.mode === "group") return groupOutput;
      if (group.mode === "password") return passOutput;
      if (group.mode === "qr") return qrOutput;
      if (group.mode === "phone") return phoneOutput;
      if (group.mode === "selo") return seloOutput;
      if (group.mode === "image") return imageOutput;
      return output;
    };
    const chooseGroup = async (id) => {
      localTool.value = id;
      localTool.dispatchEvent(new Event("change", { bubbles: true }));
      await waitFor(() => localTool.value === id && !run.disabled, `tool ${id} did not settle`);
      await delay(60);
      return groups.find((group) => group.id === id);
    };
    const chooseLeaf = async (leafId) => {
      groupTool.value = leafId;
      groupTool.dispatchEvent(new Event("change", { bubbles: true }));
      await waitFor(() => groupTool.value === leafId && !run.disabled, `operation ${leafId} did not settle`);
      await delay(60);
      return api.findTool(leafId);
    };
    const runSelected = async (active, label) => {
      const before = active.textContent.trim();
      run.click();
      await waitFor(
        () => run.disabled || active.textContent.trim() === "running..." || active.textContent.trim() !== before,
        `${label} did not start running`
      );
      await waitFor(
        () => !run.disabled && active.textContent.trim() !== "running..." && active.textContent.trim() !== before,
        `${label} did not finish running`
      );
      await delay(40);
      return active.textContent.trim();
    };
    const looksLikeError = (value) => /^(invalid|unknown|cannot|choose|error|failed)|must contain|unavailable/i.test(value);

    await chooseGroup("ids-time");
    await chooseLeaf("uuid-v4");
    const uuidOutput = await runSelected(groupOutput, "uuid-v4");
    assert(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(uuidOutput), "UUID v4 output is not valid");

    await chooseLeaf("time-convert");
    assert(groupOutput.textContent.trim() === "ready", "group output was not cleaned on operation change");
    assert(groupInput.value === "1800000000", "time converter sample was not applied");

    await chooseLeaf("inspect-uuid-v7");
    assert(groupInput.value === "015d3ef7-9800-7123-9678-90abcdef1011", "UUIDv7 sample was not applied");
    assert(!groupInput.value.includes("+55"), "stale phone input leaked into UUID inspector");

    for (const group of groups) {
      await chooseGroup(group.id);
      const active = outputFor(group);
      assert(active.textContent.trim() === "ready", `${group.id} did not clean output on selection`);

      if (group.mode === "group") {
        assert(visible(groupPanel), `${group.id} grouped panel should be visible`);
        assert(!visible(textPanel), `${group.id} should hide legacy text panel`);
        assert(!visible(optionWrap), `${group.id} should hide top option field`);
        if (group.tools.length > 1) assert(visible(groupToolField), `${group.id} operation selector should be visible`);

        for (const leafId of group.tools) {
          const leaf = await chooseLeaf(leafId);
          assert(groupOutput.textContent.trim() === "ready", `${leaf.id} did not clean output on operation selection`);
          assert(visible(explain), `${leaf.id} explanation is hidden`);
          assert(explain.textContent.trim().length > 80, `${leaf.id} explanation is too short`);
          if (leaf.mode === "generate") {
            assert(!visible(groupInputField), `${leaf.id} should hide input field`);
          } else {
            assert(visible(groupInputField), `${leaf.id} input field should be visible`);
            assert(groupInput.value === leaf.sample, `${leaf.id} sample mismatch`);
          }
          if (leaf.option) {
            assert(visible(groupOptionField), `${leaf.id} option should be visible`);
            assert(groupOptionLabel.textContent.trim() === leaf.optionLabel, `${leaf.id} option label mismatch`);
            assert(groupOption.value === leaf.optionDefault, `${leaf.id} option default mismatch`);
            assert(visible(groupOptionHelp), `${leaf.id} option help should be visible`);
          } else {
            assert(!visible(groupOptionField), `${leaf.id} option should be hidden`);
          }
          const result = await runSelected(groupOutput, leaf.id);
          assert(result && result !== "ready" && !looksLikeError(result), `${leaf.id} produced an error-like output: ${result}`);
        }
      } else if (group.mode === "image") {
        assert(!visible(textPanel), "image tool should hide generic text panel");
        assert(visible(imagePanel), "image panel should be visible");
        assert(imageOutput.textContent.trim() === "ready", "image output should be clean");
        assert(imageInfo.textContent.trim() === "choose an image", "image info should be reset");
        assert(imageDownload.hidden, "image download should be hidden after selection");
        const sourceCanvas = document.createElement("canvas");
        sourceCanvas.width = 120;
        sourceCanvas.height = 80;
        const sourceContext = sourceCanvas.getContext("2d");
        sourceContext.fillStyle = "#1f6feb";
        sourceContext.fillRect(0, 0, 120, 80);
        sourceContext.fillStyle = "#9ece6a";
        sourceContext.fillRect(24, 18, 72, 44);
        const blob = await new Promise((resolve) => sourceCanvas.toBlob(resolve, "image/png"));
        const transfer = new DataTransfer();
        transfer.items.add(new File([blob], "live-sample.png", { type: "image/png" }));
        imageFile.files = transfer.files;
        imageFile.dispatchEvent(new Event("change", { bubbles: true }));
        const imageMeta = () => {
          try { return JSON.parse(imageOutput.textContent); } catch { return {}; }
        };
        await waitFor(() => imageCanvas.width === 120 && !imageDownload.hidden && imageMeta().live === true, "image did not live-render after file selection");
        imageWidth.value = "60";
        imageWidth.dispatchEvent(new Event("input", { bubbles: true }));
        await waitFor(() => imageCanvas.width === 60 && imageCanvas.height === 40 && imageMeta().width === 60, "image did not live downscale");
        const smallRect = imageCanvas.getBoundingClientRect();
        imageWidth.value = "180";
        imageWidth.dispatchEvent(new Event("input", { bubbles: true }));
        await waitFor(() => imageCanvas.width === 180 && imageCanvas.height === 120 && imageMeta().width === 180, "image did not live upscale");
        assert(imageCanvas.getBoundingClientRect().width > smallRect.width, "resized canvas should visibly change size");
        imageFormat.value = "image/jpeg";
        imageFormat.dispatchEvent(new Event("change", { bubbles: true }));
        imageQuality.value = "0.35";
        imageQuality.dispatchEvent(new Event("input", { bubbles: true }));
        await waitFor(() => imageMeta().format === "image/jpeg" && imageMeta().quality === 0.35, "image quality/format did not update live");
        assert(!imageDownload.hidden && imageDownload.download.endsWith(".jpg"), "live image download should track format");
      } else if (group.mode === "password") {
        assert(!visible(textPanel), "password tool should hide generic text panel");
        assert(visible(passPanel), "password panel should be visible");
        const result = await runSelected(passOutput, group.id);
        assert(result.length >= 8 && !looksLikeError(result), "password generation failed");
      } else if (group.mode === "qr") {
        assert(!visible(textPanel), "QR tool should hide generic text panel");
        assert(!visible(optionWrap), "QR tool should hide generic option field");
        assert(visible(qrPanel), "QR panel should be visible");
        const result = await runSelected(qrOutput, group.id);
        assert(result.includes("qr-code.") && !looksLikeError(result), "QR export failed");
        assert(!qrDownload.hidden, "QR download link should be visible after export");
        assert(qrInfo.textContent.includes("modules"), "QR info should include module count");
        const pixels = qrCanvas.getContext("2d").getImageData(0, 0, qrCanvas.width, qrCanvas.height).data;
        let darkPixels = 0;
        for (let index = 0; index < pixels.length; index += 4) {
          if (pixels[index] < 64 && pixels[index + 1] < 64 && pixels[index + 2] < 64 && pixels[index + 3] > 0) darkPixels += 1;
          if (darkPixels > 20) break;
        }
        assert(darkPixels > 20, "QR canvas appears blank");
      } else if (group.mode === "phone") {
        assert(!visible(textPanel), "phone tool should hide generic text panel");
        assert(visible(phonePanel), "phone panel should be visible");
        assert(phoneInput.value === "+55 (11) 93039-0628", "phone sample should be formatted");
        const result = await runSelected(phoneOutput, group.id);
        assert(result.includes("DDD 11") && !looksLikeError(result), "phone lookup failed");
        assert(phoneInput.value.includes("(11)") && phoneInput.value.includes("-"), "phone input should stay human-formatted");
        assert(phoneResults.querySelectorAll(".result-card").length > 0, "phone summary card was not rendered");
        assert(phoneResults.querySelectorAll(".phone-card").length > 0, "phone cards were not rendered");
        phoneInput.value = "+91 87308 44504";
        const indiaResult = await runSelected(phoneOutput, `${group.id}-india`);
        assert(phoneInput.value === "+91 87308 44504", "non-Brazil international phone should not be reformatted");
        assert(indiaResult.includes("India +91"), "India calling code should be detected");
        assert(!indiaResult.includes("DDD 91"), "non-Brazil +91 should not be treated as Brazilian DDD");
        assert(!indiaResult.includes("\"e164\":\"+55"), "non-Brazil +91 should not produce Brazilian E.164 output");
      } else if (group.mode === "selo") {
        assert(!visible(textPanel), "selo tool should hide generic text panel");
        assert(!visible(optionWrap), "selo tool should hide generic option field");
        assert(visible(seloPanel), "selo panel should be visible");
        assert(seloKind.options.length >= 15, "selo kind list is incomplete");
        assert(seloAction.options.length >= 4, "selo action list is incomplete");
        assert(visible(seloInputField), "selo validate should show input field");
        assert(seloInput.value.includes("529"), "selo sample should be applied");
        const result = await runSelected(seloOutput, group.id);
        assert(/"valid":\s*true/.test(result) && !looksLikeError(result), "selo validation failed");
        assert(seloResults.querySelectorAll(".result-card").length > 0, "selo result card was not rendered");
        seloAction.value = "generate";
        seloAction.dispatchEvent(new Event("change", { bubbles: true }));
        await delay(80);
        assert(!visible(seloInputField), "selo generate should hide input field");
      }
    }

    notes.push(`${tools.length} tools exercised`);
    return { scenario: scenario.name, failures, notes };
  }.toString();
}

function findChrome() {
  const windowsCandidates = [
    process.env.CHROME_PATH,
    join(process.env.ProgramFiles || "", "Microsoft", "Edge", "Application", "msedge.exe"),
    join(process.env["ProgramFiles(x86)"] || "", "Microsoft", "Edge", "Application", "msedge.exe"),
    join(process.env.ProgramFiles || "", "Google", "Chrome", "Application", "chrome.exe"),
    join(process.env["ProgramFiles(x86)"] || "", "Google", "Chrome", "Application", "chrome.exe")
  ];
  const otherCandidates = [
    process.env.CHROME_PATH,
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
    "/usr/bin/google-chrome",
    "/usr/bin/chromium",
    "/usr/bin/chromium-browser",
    "/usr/bin/microsoft-edge"
  ];
  const candidates = (process.platform === "win32" ? windowsCandidates : otherCandidates).filter(Boolean);
  const found = candidates.find((candidate) => existsSync(candidate));
  if (!found) throw new Error("Chrome/Edge executable not found. Set CHROME_PATH.");
  return found;
}

async function freePort() {
  const server = createServer();
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const { port } = server.address();
  await new Promise((resolve) => server.close(resolve));
  return port;
}

async function canFetch(url) {
  try {
    const response = await fetch(url, { method: "HEAD" });
    return response.ok;
  } catch {
    return false;
  }
}

async function waitForHttp(url, timeout) {
  const started = Date.now();
  while (Date.now() - started < timeout) {
    if (await canFetch(url)) return;
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`Timed out waiting for ${url}`);
}

async function waitForBrowserEndpoint(url, processHandle, getLog, timeout) {
  const exited = new Promise((_, reject) => {
    processHandle.once("exit", (code, signal) => {
      reject(new Error(`Browser exited before DevTools became available: code=${code} signal=${signal}\n${getLog()}`));
    });
  });
  try {
    await Promise.race([waitForHttp(url, timeout), exited]);
  } catch (error) {
    if (error.message.startsWith("Timed out")) {
      throw new Error(`${error.message}\nBrowser stderr:\n${getLog()}`);
    }
    throw error;
  }
}

async function json(url) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`${url} returned ${response.status}`);
  return response.json();
}
