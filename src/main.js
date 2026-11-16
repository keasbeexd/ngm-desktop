// NGM Desktop's window: shows the app's state and calls its commands
// (src-tauri/src/lib.rs). Text only goes in with textContent.
const { invoke } = window.__TAURI__.core;
const $ = (id) => document.getElementById(id);

const ROLE = { gm: "Guild Master", officer: "officer", member: "member" };

function render(v) {
  $("error").hidden = !v.error;
  $("error").textContent = v.error ?? "";

  $("wow-none").hidden = !!v.wow;
  $("wow-path").hidden = !v.wow;
  $("wow-path").textContent = v.wow ?? "";
  $("wow-accounts").hidden = !v.wow;
  $("wow-accounts").textContent = v.accounts.length === 0
    ? "No NGM data here yet: log in to WoW with the addon once."
    : `NGM data for ${v.accounts.length} WoW account${v.accounts.length === 1 ? "" : "s"}.`;
  $("find").textContent = v.wow ? "Find again" : "Find it";

  $("connect-box").hidden = v.connected;
  $("connected-box").hidden = !v.connected;
  const list = $("guilds");
  list.replaceChildren(...v.guilds.map((g) => {
    const li = document.createElement("li");
    li.textContent = `<${g.name}> (${g.region.toUpperCase()})${g.role ? `, as ${ROLE[g.role] ?? g.role}` : ", no linked characters there now"}`;
    return li;
  }));
  if (v.connected && v.guilds.length === 0) {
    const li = document.createElement("li");
    li.textContent = "None yet";
    list.append(li);
  }
}

async function run(cmd, args, button) {
  document.querySelectorAll("button").forEach((b) => (b.disabled = true));
  try {
    render(await invoke(cmd, args));
  } catch (e) {
    $("error").hidden = false;
    $("error").textContent = String(e);
  } finally {
    document.querySelectorAll("button").forEach((b) => (b.disabled = false));
    button?.blur();
  }
}

$("find").addEventListener("click", (e) => run("find_wow", {}, e.target));
$("choose").addEventListener("click", (e) => run("choose_wow", {}, e.target));
$("disconnect").addEventListener("click", (e) => run("disconnect", {}, e.target));
$("connect").addEventListener("submit", (e) => {
  e.preventDefault();
  run("connect", { code: $("code").value }).then(() => ($("code").value = ""));
});

run("state", {});
