use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct XtreamSession {
    pub server: String, // e.g. http://host:port  (no trailing slash)
    pub username: String,
    pub password: String,
}

impl XtreamSession {
    fn base(&self) -> String {
        self.server.trim_end_matches('/').to_string()
    }

    fn player_api_url(&self, action: Option<&str>, extra: &[(&str, &str)]) -> String {
        let mut url = format!(
            "{}/player_api.php?username={}&password={}",
            self.base(),
            urlencoding::encode(&self.username),
            urlencoding::encode(&self.password)
        );
        if let Some(a) = action {
            url.push_str(&format!("&action={}", a));
        }
        for (k, v) in extra {
            url.push_str(&format!("&{}={}", k, urlencoding::encode(v)));
        }
        url
    }

    /// Authenticate and return the raw account/server info JSON from Xtream Codes.
    pub async fn authenticate(&self) -> anyhow::Result<Value> {
        let url = self.player_api_url(None, &[]);
        let client = http_client()?;
        let resp = client.get(&url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("Server responded with status {}", resp.status());
        }
        let json: Value = resp.json().await?;
        let auth_ok = json
            .get("user_info")
            .and_then(|u| u.get("auth"))
            .and_then(|a| a.as_i64())
            .unwrap_or(0);
        if auth_ok != 1 {
            anyhow::bail!("Invalid credentials or account inactive");
        }
        Ok(json)
    }

    pub async fn get_live_categories(&self) -> anyhow::Result<Value> {
        self.get_json("get_live_categories", &[]).await
    }

    pub async fn get_live_streams(&self, category_id: Option<&str>) -> anyhow::Result<Value> {
        let extra = category_id
            .map(|c| vec![("category_id", c)])
            .unwrap_or_default();
        self.get_json("get_live_streams", &extra).await
    }

    pub async fn get_vod_categories(&self) -> anyhow::Result<Value> {
        self.get_json("get_vod_categories", &[]).await
    }

    pub async fn get_vod_streams(&self, category_id: Option<&str>) -> anyhow::Result<Value> {
        let extra = category_id
            .map(|c| vec![("category_id", c)])
            .unwrap_or_default();
        self.get_json("get_vod_streams", &extra).await
    }

    pub async fn get_vod_info(&self, vod_id: &str) -> anyhow::Result<Value> {
        self.get_json("get_vod_info", &[("vod_id", vod_id)]).await
    }

    pub async fn get_series_categories(&self) -> anyhow::Result<Value> {
        self.get_json("get_series_categories", &[]).await
    }

    pub async fn get_series(&self, category_id: Option<&str>) -> anyhow::Result<Value> {
        let extra = category_id
            .map(|c| vec![("category_id", c)])
            .unwrap_or_default();
        self.get_json("get_series", &extra).await
    }

    pub async fn get_series_info(&self, series_id: &str) -> anyhow::Result<Value> {
        self.get_json("get_series_info", &[("series_id", series_id)])
            .await
    }

    async fn get_json(&self, action: &str, extra: &[(&str, &str)]) -> anyhow::Result<Value> {
        let url = self.player_api_url(Some(action), extra);
        let client = http_client()?;
        let resp = client.get(&url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("Server responded with status {}", resp.status());
        }
        let json: Value = resp.json().await?;
        Ok(json)
    }

    /// Build a direct stream URL. kind: "live" | "movie" | "series"
    pub fn stream_url(&self, kind: &str, stream_id: &str, ext: &str) -> String {
        format!(
            "{}/{}/{}/{}/{}.{}",
            self.base(),
            kind,
            urlencoding::encode(&self.username),
            urlencoding::encode(&self.password),
            stream_id,
            ext
        )
    }
}

fn http_client() -> anyhow::Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .danger_accept_invalid_certs(false)
        .timeout(std::time::Duration::from_secs(20))
        .user_agent("EliteStocksTV/1.0 (Windows; Tauri)")
        .build()?)
}
