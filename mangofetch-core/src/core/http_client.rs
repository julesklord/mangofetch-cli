use std::sync::LazyLock;
use std::sync::RwLock;

use crate::models::settings::ProxySettings;

static GLOBAL_PROXY: LazyLock<RwLock<ProxySettings>> =
    LazyLock::new(|| RwLock::new(ProxySettings::default()));

pub fn init_proxy(proxy: ProxySettings) {
    if let Ok(mut guard) = GLOBAL_PROXY.write() {
        *guard = proxy;
    }
}

pub fn get_proxy_snapshot() -> ProxySettings {
    GLOBAL_PROXY.read().map(|g| g.clone()).unwrap_or_default()
}

pub fn proxy_url() -> Option<String> {
    let proxy = get_proxy_snapshot();
    if !proxy.enabled || proxy.host.is_empty() {
        return None;
    }
    let scheme = match proxy.proxy_type.as_str() {
        "socks5" => "socks5",
        "https" => "https",
        _ => "http",
    };
    if !proxy.username.is_empty() {
        Some(format!(
            "{}://{}:{}@{}:{}",
            scheme, proxy.username, proxy.password, proxy.host, proxy.port
        ))
    } else {
        Some(format!("{}://{}:{}", scheme, proxy.host, proxy.port))
    }
}

pub fn apply_proxy(
    builder: reqwest::ClientBuilder,
    proxy: &ProxySettings,
) -> reqwest::ClientBuilder {
    if !proxy.enabled || proxy.host.is_empty() {
        return builder;
    }
    let scheme = match proxy.proxy_type.as_str() {
        "socks5" => "socks5",
        "https" => "https",
        _ => "http",
    };
    let proxy_url = if !proxy.username.is_empty() {
        format!(
            "{}://{}:{}@{}:{}",
            scheme, proxy.username, proxy.password, proxy.host, proxy.port
        )
    } else {
        format!("{}://{}:{}", scheme, proxy.host, proxy.port)
    };
    match reqwest::Proxy::all(&proxy_url) {
        Ok(p) => builder.proxy(p),
        Err(e) => {
            tracing::warn!("Invalid proxy URL: {}", e);
            builder
        }
    }
}

pub fn apply_global_proxy(builder: reqwest::ClientBuilder) -> reqwest::ClientBuilder {
    let proxy = get_proxy_snapshot();
    apply_proxy(builder, &proxy)
}

pub fn inject_ua_header(headers: &mut reqwest::header::HeaderMap, opts_ua: Option<&str>) {
    if let Some(ua) = opts_ua {
        if let Ok(v) = reqwest::header::HeaderValue::from_str(ua) {
            headers.insert(reqwest::header::USER_AGENT, v);
        }
    }
}

pub fn ua_header_map(opts_ua: Option<&str>) -> Option<reqwest::header::HeaderMap> {
    let ua = opts_ua?;
    let value = reqwest::header::HeaderValue::from_str(ua).ok()?;
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(reqwest::header::USER_AGENT, value);
    Some(headers)
}

pub async fn download_with_progress<F>(url: &str, mut on_progress: F) -> anyhow::Result<Vec<u8>>
where
    F: FnMut(f32) + Send,
{
    let client = apply_global_proxy(reqwest::Client::builder())
        .timeout(std::time::Duration::from_secs(300))
        .build()?;

    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("HTTP error: {}", response.status()));
    }

    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut buffer = Vec::with_capacity(total_size as usize);

    use futures::StreamExt;
    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        buffer.extend_from_slice(&chunk);
        downloaded += chunk.len() as u64;

        if total_size > 0 {
            let percent = (downloaded as f32 / total_size as f32) * 100.0;
            on_progress(percent);
        }
    }

    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::settings::ProxySettings;

    #[test]
    fn test_proxy_url_generation() {
        // Disabled
        init_proxy(ProxySettings {
            enabled: false,
            proxy_type: "http".into(),
            host: "127.0.0.1".into(),
            port: 8080,
            username: "".into(),
            password: "".into(),
        });
        assert_eq!(proxy_url(), None);

        // Missing host
        init_proxy(ProxySettings {
            enabled: true,
            proxy_type: "http".into(),
            host: "".into(),
            port: 8080,
            username: "".into(),
            password: "".into(),
        });
        assert_eq!(proxy_url(), None);

        // HTTP proxy without auth
        init_proxy(ProxySettings {
            enabled: true,
            proxy_type: "http".into(),
            host: "127.0.0.1".into(),
            port: 8080,
            username: "".into(),
            password: "".into(),
        });
        assert_eq!(proxy_url(), Some("http://127.0.0.1:8080".into()));

        // SOCKS5 proxy with auth
        init_proxy(ProxySettings {
            enabled: true,
            proxy_type: "socks5".into(),
            host: "127.0.0.1".into(),
            port: 1080,
            username: "user".into(),
            password: "password".into(),
        });
        assert_eq!(
            proxy_url(),
            Some("socks5://user:password@127.0.0.1:1080".into())
        );

        // HTTPS proxy without auth
        init_proxy(ProxySettings {
            enabled: true,
            proxy_type: "https".into(),
            host: "127.0.0.1".into(),
            port: 443,
            username: "".into(),
            password: "".into(),
        });
        assert_eq!(proxy_url(), Some("https://127.0.0.1:443".into()));

        // Unknown proxy type (falls back to http)
        init_proxy(ProxySettings {
            enabled: true,
            proxy_type: "unknown".into(),
            host: "127.0.0.1".into(),
            port: 8080,
            username: "".into(),
            password: "".into(),
        });
        assert_eq!(proxy_url(), Some("http://127.0.0.1:8080".into()));
    }
}
