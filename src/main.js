// NGM Desktop's window: shows the app's state and calls its commands
// (src-tauri/src/lib.rs). Text only goes in with textContent.
const { invoke } = window.__TAURI__.core;
const $ = (id) => document.getElementById(id);

const ROLE = { gm: "Guild Master", officer: "officer", member: "member" };

function render(v) {
  $("error").hidden = !v.error;
  $("error").textContent = v.error ?? "";

  $("wow-none").hidden = !!v.wow;
  $("wow-label").hidden = !v.wow;
  $("wow-path").hidden = !v.wow;
  $("wow-path").textContent = v.wow ?? "";
  $("wow-accounts").hidden = !v.wow;
  $("wow-accounts").textContent = v.accounts.length === 0
    ? "No NGM data here yet: log in to WoW with the addon once."
    : `NGM data for ${v.accounts.length} WoW account${v.accounts.length === 1 ? "" : "s"}.`;
  $("choose").textContent = v.wow ? "Change folder" : "Choose folder";

  $("helper-new").hidden = !v.helper_new;
  $("send").hidden = !(v.connected && v.wow);
  if (v.last) {
    const when = new Date(v.last.at * 1000).toLocaleString(undefined, { weekday: "short", hour: "2-digit", minute: "2-digit" });
    $("last").textContent = v.last.ok ? `Last sent ${when}:` : `Last try ${when} didn't go through:`;
    $("last").className = v.last.ok ? "dim" : "bad";
    $("last-lines").replaceChildren(...v.last.lines.map((l) => {
      const li = document.createElement("li");
      li.textContent = l;
      return li;
    }));
  }

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

$("choose").addEventListener("click", (e) => run("choose_wow", {}, e.target));
$("disconnect").addEventListener("click", (e) => run("disconnect", {}, e.target));
$("send").addEventListener("click", (e) => run("sync_now", {}, e.target));
// a send that happened by itself (after a logout): show it
window.__TAURI__.event.listen("synced", () => run("state", {}));
$("connect").addEventListener("submit", (e) => {
  e.preventDefault();
  run("connect", { code: $("code").value }).then(() => ($("code").value = ""));
});

// first start: look for the game by itself (the button is only for changing it)
invoke("state").then((v) => (v.wow ? render(v) : run("find_wow", {}))).catch(() => run("state", {}));
