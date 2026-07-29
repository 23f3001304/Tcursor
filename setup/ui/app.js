const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const win = window.__TAURI__.window.getCurrentWindow();

const $ = (id) => document.getElementById(id);
const show = (id) =>
  document.querySelectorAll(".screen").forEach((s) => s.classList.toggle("active", s.id === id));

const setProgress = (pct, stage) => {
  $("fill").style.width = pct + "%";
  $("status").textContent = stage;
};
const fail = (msg) => {
  $("error-msg").textContent = msg || "Something went wrong.";
  show("error");
};

async function install() {
  show("installing");
  setProgress(3, "Starting…");
  try {
    await invoke("start_install");
  } catch (e) {
    fail(String(e));
  }
}

document.querySelectorAll("[data-close]").forEach((b) => (b.onclick = () => win.close()));
$("install").onclick = install;
$("retry").onclick = install;
$("launch").onclick = async () => {
  try {
    await invoke("launch_app");
  } finally {
    win.close();
  }
};
$("license-link").onclick = (e) => {
  e.preventDefault();
  $("license-overlay").classList.add("open");
};
$("license-close").onclick = () => $("license-overlay").classList.remove("open");

listen("install://tick", (e) => setProgress(e.payload.pct, e.payload.stage));
listen("install://done", (e) => {
  if (e.payload.ok) {
    setProgress(100, "Done");
    setTimeout(() => show("done"), 500);
  } else {
    fail(e.payload.message);
  }
});
