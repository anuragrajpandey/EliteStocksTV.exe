import { Api, listen } from "./api.js";

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------
const $ = (sel) => document.querySelector(sel);
const el = (tag, cls, html) => {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (html !== undefined) e.innerHTML = html;
  return e;
};
const showView = (id) => {
  document.querySelectorAll(".view").forEach((v) => v.classList.remove("active"));
  $(id).classList.add("active");
};
const loading = (on) => ($("#loading").hidden = !on);
const toast = (msg, ms = 3000) => {
  const t = $("#toast");
  t.textContent = msg;
  t.hidden = false;
  clearTimeout(toast._t);
  toast._t = setTimeout(() => (t.hidden = true), ms);
};
const asArray = (v) => (Array.isArray(v) ? v : v && typeof v === "object" ? Object.values(v) : []);

// ---------------------------------------------------------------------------
// Titlebar
// ---------------------------------------------------------------------------
$("#tb-min").onclick = () => Api.minimize();
$("#tb-max").onclick = () => Api.toggleMaximize();
$("#tb-close").onclick = () => Api.close();

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------
const REMEMBER_KEY = "elitestockstv.creds";

function saveCreds(server, username, password) {
  localStorage.setItem(REMEMBER_KEY, JSON.stringify({ server, username, password }));
}
function clearCreds() {
  localStorage.removeItem(REMEMBER_KEY);
}
function loadCreds() {
  try {
    return JSON.parse(localStorage.getItem(REMEMBER_KEY) || "null");
  } catch {
    return null;
  }
}

async function doLogin(server, username, password, remember) {
  const btn = $("#login-btn");
  const errEl = $("#login-error");
  errEl.hidden = true;
  btn.disabled = true;
  btn.textContent = "Signing in...";
  try {
    const info = await Api.login(server, username, password);
    if (remember) saveCreds(server, username, password);
    else clearCreds();
    const name = info?.user_info?.username || username;
    toast(`Welcome, ${name}`);
    showView("#view-home");
    initHome();
  } catch (e) {
    errEl.textContent = typeof e === "string" ? e : e?.message || "Sign in failed. Check your server, username and password.";
    errEl.hidden = false;
  } finally {
    btn.disabled = false;
    btn.textContent = "Sign In";
  }
}

$("#login-form").addEventListener("submit", (ev) => {
  ev.preventDefault();
  const server = $("#f-server").value.trim();
  const username = $("#f-username").value.trim();
  const password = $("#f-password").value;
  const remember = $("#f-remember").checked;
  if (!server || !username || !password) return;
  doLogin(server, username, password, remember);
});

$("#nav-logout").onclick = async () => {
  await Api.logout();
  clearCreds();
  showView("#view-login");
};

// Try auto sign-in with remembered credentials
(function tryAutoLogin() {
  const creds = loadCreds();
  if (creds?.server && creds?.username && creds?.password) {
    $("#f-server").value = creds.server;
    $("#f-username").value = creds.username;
    doLogin(creds.server, creds.username, creds.password, true);
  }
})();

// ---------------------------------------------------------------------------
// Home / catalog
// ---------------------------------------------------------------------------
let homeInitialized = false;
let currentTab = "live";
let liveCategories = [];
let vodCategories = [];
let seriesCategories = [];
let activeLiveCategory = null;

document.querySelectorAll(".nav-tab").forEach((btn) => {
  btn.addEventListener("click", () => {
    document.querySelectorAll(".nav-tab").forEach((b) => b.classList.remove("active"));
    btn.classList.add("active");
    currentTab = btn.dataset.tab;
    renderTab(currentTab);
  });
});

$("#search-input").addEventListener("input", (e) => {
  const q = e.target.value.trim().toLowerCase();
  document.querySelectorAll(".card").forEach((card) => {
    const name = (card.dataset.name || "").toLowerCase();
    card.style.display = !q || name.includes(q) ? "" : "none";
  });
});

async function initHome() {
  if (homeInitialized) {
    renderTab(currentTab);
    return;
  }
  homeInitialized = true;
  loading(true);
  try {
    const [live, vod, series] = await Promise.all([
      Api.liveCategories().catch(() => []),
      Api.vodCategories().catch(() => []),
      Api.seriesCategories().catch(() => []),
    ]);
    liveCategories = asArray(live);
    vodCategories = asArray(vod);
    seriesCategories = asArray(series);
    renderTab(currentTab);
  } catch (e) {
    toast("Could not load your catalog. Please check your connection.");
  } finally {
    loading(false);
  }
}

async function renderTab(tab) {
  const container = $("#rows-container");
  container.innerHTML = "";
  if (tab === "live") await renderLiveTab(container);
  else if (tab === "movies") await renderRowsTab(container, "movie");
  else await renderRowsTab(container, "series");
}

// ---- Live TV: category rail + grid ----
async function renderLiveTab(container) {
  const rail = el("div", "cat-rail");
  const allChip = el("button", "cat-chip active", "All Channels");
  allChip.onclick = () => selectLiveCategory(null, allChip);
  rail.appendChild(allChip);
  liveCategories.forEach((c) => {
    const chip = el("button", "cat-chip", escapeHtml(c.category_name || "Category"));
    chip.onclick = () => selectLiveCategory(c.category_id, chip);
    rail.appendChild(chip);
  });
  container.appendChild(rail);

  const grid = el("div", "row-track", "");
  grid.style.flexWrap = "wrap";
  grid.style.padding = "6px 44px 40px";
  grid.id = "live-grid";
  container.appendChild(grid);

  await loadLiveGrid(null);

  function selectLiveCategory(catId, chipEl) {
    rail.querySelectorAll(".cat-chip").forEach((c) => c.classList.remove("active"));
    chipEl.classList.add("active");
    loadLiveGrid(catId);
  }
}

async function loadLiveGrid(categoryId) {
  activeLiveCategory = categoryId;
  const grid = $("#live-grid");
  if (!grid) return;
  grid.innerHTML = "Loading...";
  try {
    const streams = asArray(await Api.liveStreams(categoryId));
    grid.innerHTML = "";
    if (!streams.length) {
      grid.appendChild(el("div", "row-empty", "No channels in this category."));
      return;
    }
    streams.forEach((s) => grid.appendChild(channelCard(s)));
    updateHeroFromLive(streams[0]);
  } catch (e) {
    grid.innerHTML = "";
    grid.appendChild(el("div", "row-empty", "Failed to load channels."));
  }
}

function channelCard(stream) {
  const card = el("div", "card channel");
  card.dataset.name = stream.name || "";
  const img = stream.stream_icon
    ? `<img class="card-img" src="${escapeAttr(stream.stream_icon)}" onerror="this.style.display='none'" />`
    : `<div class="card-img"></div>`;
  card.innerHTML = `
    <span class="card-live-badge">LIVE</span>
    ${img}
    <div class="card-body">
      <div class="card-name">${escapeHtml(stream.name || "Channel")}</div>
      <div class="card-sub">${escapeHtml(stream.epg_channel_id || "")}</div>
    </div>`;
  card.onclick = () => playLive(stream);
  return card;
}

async function playLive(stream) {
  loading(true);
  try {
    const url = await Api.streamUrl("live", stream.stream_id, "m3u8");
    await Api.play([{ url, title: stream.name || "Live TV" }]);
  } catch (e) {
    toast("Could not start playback for this channel.");
  } finally {
    loading(false);
  }
}

function updateHeroFromLive(stream) {
  if (!stream) return;
  $("#hero-meta").textContent = "LIVE TV";
  $("#hero-title").textContent = stream.name || "EliteStocks TV";
  $("#hero-desc").textContent = "Watch live, right now.";
  $("#hero-backdrop").style.backgroundImage = stream.stream_icon ? `url("${stream.stream_icon}")` : "";
  $("#hero-play").onclick = () => playLive(stream);
  $("#hero-info").onclick = () => playLive(stream);
}

// ---- Movies / TV Shows: rows per category ----
async function renderRowsTab(container, kind) {
  const cats = kind === "movie" ? vodCategories : seriesCategories;
  if (!cats.length) {
    container.appendChild(el("div", "row-empty", "No categories found."));
    return;
  }

  let heroSet = false;
  // Cap the number of rows fetched up-front to keep things snappy; still real data.
  const toShow = cats.slice(0, 16);
  for (const cat of toShow) {
    const items = asArray(
      kind === "movie" ? await Api.vodStreams(cat.category_id).catch(() => []) : await Api.seriesList(cat.category_id).catch(() => [])
    );
    if (!items.length) continue;

    if (!heroSet) {
      updateHeroFromItem(items[0], kind);
      heroSet = true;
    }

    const row = el("div", "row");
    row.appendChild(el("h3", "row-title", escapeHtml(cat.category_name || "Category")));
    const track = el("div", "row-track");
    items.forEach((item) => track.appendChild(posterCard(item, kind)));
    row.appendChild(track);
    container.appendChild(row);
  }
}

function posterCard(item, kind) {
  const name = kind === "movie" ? item.name : item.name;
  const img = item.stream_icon || item.cover || "";
  const card = el("div", "card");
  card.dataset.name = name || "";
  card.innerHTML = `
    ${img ? `<img class="card-img" src="${escapeAttr(img)}" onerror="this.style.display='none'" />` : `<div class="card-img"></div>`}
    <div class="card-body">
      <div class="card-name">${escapeHtml(name || "")}</div>
      <div class="card-sub">${escapeHtml(item.rating ? "★ " + item.rating : "")}</div>
    </div>`;
  card.onclick = () => openDetail(item, kind);
  return card;
}

function updateHeroFromItem(item, kind) {
  $("#hero-meta").textContent = kind === "movie" ? "MOVIE" : "SERIES";
  $("#hero-title").textContent = item.name || "EliteStocks TV";
  $("#hero-desc").textContent = item.plot || "";
  const backdrop = item.backdrop_path?.[0] || item.stream_icon || item.cover || "";
  $("#hero-backdrop").style.backgroundImage = backdrop ? `url("${backdrop}")` : "";
  $("#hero-play").onclick = () => openDetail(item, kind, true);
  $("#hero-info").onclick = () => openDetail(item, kind, false);
}

// ---------------------------------------------------------------------------
// Detail modal (movie or series)
// ---------------------------------------------------------------------------
$("#detail-close").onclick = closeDetail;
$("#detail-close-backdrop").onclick = closeDetail;
function closeDetail() {
  $("#detail-modal").hidden = true;
}

async function openDetail(item, kind, autoplay = false) {
  $("#detail-modal").hidden = false;
  $("#detail-title").textContent = item.name || "";
  $("#detail-desc").textContent = "Loading details...";
  $("#detail-seasons").hidden = true;
  $("#detail-seasons").innerHTML = "";
  $("#detail-meta").textContent = "";
  const backdrop = item.backdrop_path?.[0] || item.stream_icon || item.cover || "";
  $("#detail-hero").style.backgroundImage = backdrop ? `url("${backdrop}")` : "";

  if (kind === "movie") {
    $("#detail-play").onclick = () => playMovie(item);
    if (autoplay) playMovie(item);
    try {
      const info = await Api.vodInfo(item.stream_id);
      const plot = info?.info?.plot || item.plot || "No description available.";
      $("#detail-desc").textContent = plot;
      const meta = [info?.info?.releasedate, info?.info?.duration, info?.info?.genre].filter(Boolean).join("  •  ");
      $("#detail-meta").textContent = meta;
    } catch {
      $("#detail-desc").textContent = item.plot || "No description available.";
    }
  } else {
    $("#detail-desc").textContent = "Loading episodes...";
    try {
      const info = await Api.seriesInfo(item.series_id);
      const plot = info?.info?.plot || item.plot || "No description available.";
      $("#detail-desc").textContent = plot;
      const meta = [info?.info?.releaseDate, info?.info?.genre].filter(Boolean).join("  •  ");
      $("#detail-meta").textContent = meta;
      renderSeasons(info, item);
    } catch {
      $("#detail-desc").textContent = item.plot || "No description available.";
    }
  }
}

async function playMovie(item) {
  loading(true);
  try {
    const info = await Api.vodInfo(item.stream_id).catch(() => null);
    const ext = info?.movie_data?.container_extension || item.container_extension || "mp4";
    const url = await Api.streamUrl("movie", item.stream_id, ext);
    await Api.play([{ url, title: item.name || "Movie" }]);
    closeDetail();
  } catch (e) {
    toast("Could not start playback for this title.");
  } finally {
    loading(false);
  }
}

function renderSeasons(info, seriesItem) {
  const episodesBySeason = info?.episodes || {};
  const seasonNums = Object.keys(episodesBySeason).sort((a, b) => Number(a) - Number(b));
  const wrap = $("#detail-seasons");
  wrap.hidden = false;
  wrap.innerHTML = "";

  if (!seasonNums.length) {
    wrap.appendChild(el("div", "row-empty", "No episodes found for this series."));
    return;
  }

  const select = el("select");
  seasonNums.forEach((s) => {
    const opt = el("option", "", `Season ${s}`);
    opt.value = s;
    select.appendChild(opt);
  });
  wrap.appendChild(select);

  const list = el("div", "ep-list");
  wrap.appendChild(list);

  const renderList = (season) => {
    list.innerHTML = "";
    const episodes = episodesBySeason[season] || [];
    episodes.forEach((ep, idx) => {
      const row = el("div", "ep-row");
      const thumb = ep.info?.movie_image || seriesItem.cover || "";
      row.innerHTML = `
        <div class="ep-num">${ep.episode_num ?? idx + 1}</div>
        ${thumb ? `<img class="ep-thumb" src="${escapeAttr(thumb)}" onerror="this.style.display='none'" />` : `<div class="ep-thumb"></div>`}
        <div class="ep-info">
          <div class="ep-title">${escapeHtml(ep.title || "Episode " + (ep.episode_num ?? idx + 1))}</div>
          <div class="ep-desc">${escapeHtml(ep.info?.plot || "")}</div>
        </div>
        <div class="ep-play">&#9654;</div>`;
      row.onclick = () => playEpisode(episodes, idx, seriesItem.name);
      list.appendChild(row);
    });
  };

  select.onchange = () => renderList(select.value);
  select.value = seasonNums[0];
  renderList(seasonNums[0]);
}

async function playEpisode(episodesInSeason, startIdx, seriesName) {
  loading(true);
  try {
    // Build a full playlist starting at the chosen episode so mpv/uosc's
    // native "next episode" / playlist menu works for the rest of the season.
    const items = [];
    for (let i = startIdx; i < episodesInSeason.length; i++) {
      const ep = episodesInSeason[i];
      const ext = ep.container_extension || "mp4";
      const url = await Api.streamUrl("series", ep.id, ext);
      const title = `${seriesName} - S${ep.season ?? ""}E${ep.episode_num ?? i + 1} ${ep.title || ""}`.trim();
      items.push({ url, title });
    }
    await Api.play(items);
    closeDetail();
  } catch (e) {
    toast("Could not start playback for this episode.");
  } finally {
    loading(false);
  }
}

// ---------------------------------------------------------------------------
// Player lifecycle events from the Rust backend
// ---------------------------------------------------------------------------
listen("player-closed", () => {
  // mpv exited (user closed the player) - main window is already re-shown by
  // the backend; nothing else to do here besides making sure we're on Home.
});

// ---------------------------------------------------------------------------
// Utils
// ---------------------------------------------------------------------------
function escapeHtml(str) {
  return String(str ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
}
function escapeAttr(str) {
  return String(str ?? "").replace(/"/g, "&quot;");
}
