// Thin wrapper around window.__TAURI__ (enabled via app.withGlobalTauri).
// Keeps the rest of the frontend free of import/bundler concerns.

const T = window.__TAURI__;

export const invoke = (cmd, args) => T.core.invoke(cmd, args);
export const listen = (event, cb) => T.event.listen(event, cb);

export const Api = {
  // --- auth ---
  login: (server, username, password) =>
    invoke("xtream_login", { server, username, password }),
  logout: () => invoke("xtream_logout"),

  // --- catalog ---
  liveCategories: () => invoke("get_live_categories"),
  liveStreams: (category_id) => invoke("get_live_streams", { categoryId: category_id ?? null }),
  vodCategories: () => invoke("get_vod_categories"),
  vodStreams: (category_id) => invoke("get_vod_streams", { categoryId: category_id ?? null }),
  vodInfo: (vod_id) => invoke("get_vod_info", { vodId: String(vod_id) }),
  seriesCategories: () => invoke("get_series_categories"),
  seriesList: (category_id) => invoke("get_series_list", { categoryId: category_id ?? null }),
  seriesInfo: (series_id) => invoke("get_series_info", { seriesId: String(series_id) }),
  streamUrl: (kind, stream_id, ext) =>
    invoke("build_stream_url", { kind, streamId: String(stream_id), ext }),

  // --- player ---
  play: (items) => invoke("player_play", { items }),
  stop: () => invoke("player_stop"),

  // --- window chrome ---
  minimize: () => invoke("window_minimize"),
  toggleMaximize: () => invoke("window_toggle_maximize"),
  close: () => invoke("window_close"),
};
