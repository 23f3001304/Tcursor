// Paint what Rust reports. Every decision (percent, phase, status wording, the log, whether a
// close is allowed) is made behind a command or an event; this file only moves it onto the page.
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const win = window.__TAURI__.window.getCurrentWindow();

const $ = (id) => document.getElementById(id);
let info = { version: "", destination: "", log_path: "", expected_mb: 0 };
let openTimer = null;

const LICENSE_NOTE = 'By installing you agree to the <a href="#" data-license>license</a>.';

function show(id) {
  document.querySelectorAll(".screen").forEach((s) => s.classList.toggle("is-on", s.id === id));
  const note = $("rail-note");
  if (id === "s-welcome") note.innerHTML = LICENSE_NOTE;
  else note.textContent = id === "s-error" ? "Log: " + info.log_path : "";
}

function paint(percent, phase, status) {
  $("percent").textContent = percent;
  $("fill").style.width = percent + "%";
  $("status").textContent = status;
  $("track").dataset.phase = phase;
  $("track").setAttribute("aria-valuenow", percent);
}

function fail(message) {
  clearTimeout(openTimer);
  $("confirm").hidden = true;
  $("message").textContent = message || "The installer stopped without saying why.";
  show("s-error");
}

async function install() {
  paint(0, "preparing", "Preparing");
  show("s-installing");
  try {
    await invoke("start_install");
  } catch (e) {
    fail(String(e));
  }
}

async function openApp() {
  clearTimeout(openTimer);
  try {
    await invoke("launch_app");
    await win.close();
  } catch (e) {
    fail(String(e));
  }
}

async function openSheet(event) {
  event.preventDefault();
  if (!$("sheet-body").textContent) $("sheet-body").textContent = await invoke("license_text");
  $("sheet").hidden = false;
}

async function copyText(text) {
  try {
    await navigator.clipboard.writeText(text);
    return;
  } catch {
    // WebView2 serves this window over http://tauri.localhost, which is not a secure context
    // everywhere the clipboard API insists on one.
  }
  const field = document.createElement("textarea");
  field.value = text;
  field.style.cssText = "position:fixed;opacity:0";
  document.body.appendChild(field);
  field.select();
  document.execCommand("copy");
  field.remove();
}

$("install").onclick = install;
$("retry").onclick = install;
$("open").onclick = openApp;
$("sheet-close").onclick = () => ($("sheet").hidden = true);
$("confirm-no").onclick = () => ($("confirm").hidden = true);
$("confirm-yes").onclick = () => invoke("allow_close");
$("rail-note").onclick = (e) => e.target.matches("[data-license]") && openSheet(e);
document.querySelectorAll("[data-close]").forEach((b) => (b.onclick = () => win.close()));

$("copy").onclick = async () => {
  await copyText(await invoke("copy_log"));
  $("copy").textContent = "Copied";
  setTimeout(() => ($("copy").textContent = "Copy log"), 1600);
};

addEventListener("keydown", (e) => {
  if (e.key !== "Escape") return;
  if (!$("sheet").hidden) $("sheet").hidden = true;
  else if (!$("confirm").hidden) $("confirm").hidden = true;
  else win.close();
});

listen("install://tick", (e) => paint(e.payload.percent, e.payload.phase, e.payload.status));
listen("install://done", (e) => {
  if (!e.payload.ok) return fail(e.payload.message);
  // Let the bar finish its run to 100 before the screen changes, then hold the checkmark for a
  // beat and open the app. One variable, so Esc or the button cancels whichever is pending.
  openTimer = setTimeout(() => {
    show("s-done");
    openTimer = setTimeout(openApp, 1100);
  }, 260);
});
// The window would not close on its own mid-install, so ask in a line rather than an OS dialog.
listen("setup://close-request", () => ($("confirm").hidden = false));

invoke("setup_info").then((got) => {
  info = got;
  $("version").textContent = info.version;
  $("destination").textContent = info.destination;
  $("destination").title = info.destination;
  if (info.expected_mb > 0) {
    $("welcome-size").textContent = `About ${info.expected_mb} MB, installed for you only.`;
  }
  show("s-welcome");
});
