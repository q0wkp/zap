//! models.dev 数据源接入。
//!
//! 在用户打开 Providers 设置页时,后台异步拉取 `https://models.dev/api.json`,
//! 缓存到 `${cache_dir}/models-dev.json`。下一次启动直接读缓存,
//! 缓存命中且未过 TTL(默认 24h) 不再发请求;过期/缺失时再去拉。
//!
//! 数据结构对齐 opencode 的 `provider/models.ts`:顶层是
//! `{ <provider_id>: Provider }`,Provider 含 `models: { <model_id>: Model }`。
//! 我们只关心 UI "快速选择" 需要的几个字段:
//! - provider: id / name / api / env(暗示需要哪个 env var)
//! - model:    id / name / limit.context / limit.output / reasoning / tool_call
//!
//! 没列出的字段一律走 `serde(default)` + `#[allow(dead_code)]` 容忍。
//!
//! 设计取舍:**同步缓存读、异步网络拉**。读侧给 UI 用,要快;
//! 拉侧后台 spawn,失败不弹错只 log,缓存读不到就给空数据,UI 展示
//! "暂未拉取到 models.dev,请检查网络"。
//!
//! A copy of the catalog is built into the binary (`app/resources/models_dev/`, written by
//! `script/update_models_dev_catalog`). It stands in when nothing was downloaded yet, when
//! the download fails, and when downloading is turned off
//! (`agents.byop.models_dev.download`); the download URL is `agents.byop.models_dev.url`.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::sync::RwLock;
use std::time::{Duration, SystemTime};

use http_client::Client;
use serde::{Deserialize, Serialize};

/// Where the catalog is downloaded from unless `agents.byop.models_dev.url` says otherwise.
pub const DEFAULT_CATALOG_URL: &str = "https://models.dev/api.json";
/// The built-in catalog: the fields of [`Catalog`] that Zap reads, as gzipped JSON.
const BUNDLED_CATALOG_GZ: &[u8] = include_bytes!("../../../resources/models_dev/catalog.json.gz");
/// `key=value` lines saying where and when the built-in catalog was taken.
const BUNDLED_CATALOG_INFO: &str = include_str!("../../../resources/models_dev/catalog.txt");
const CACHE_FILENAME: &str = "models-dev.json";
const CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const FETCH_TIMEOUT: Duration = Duration::from_secs(15);

/// `models.dev` 顶层数据 — provider_id → Provider。
pub type Catalog = BTreeMap<String, Provider>;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Provider {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    /// 上游 API base URL,例如 `https://api.deepseek.com/v1`。
    #[serde(default)]
    pub api: Option<String>,
    /// 该 provider 通常需要的环境变量名,例如 `["DEEPSEEK_API_KEY"]`。
    #[serde(default)]
    pub env: Vec<String>,
    /// 可用模型,key 为模型 id。
    #[serde(default)]
    pub models: BTreeMap<String, Model>,
    /// 文档 URL(部分 provider 有)。
    #[serde(default)]
    pub doc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Model {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub family: Option<String>,
    #[serde(default)]
    pub release_date: Option<String>,
    #[serde(default)]
    pub reasoning: bool,
    #[serde(default = "default_true")]
    pub tool_call: bool,
    /// 是否支持文件附件(attachment 字段,与 modalities 互补:
    /// modalities 描述 native 多模态;attachment 涵盖 PDF / 通用文件附件协议)。
    #[serde(default)]
    pub attachment: bool,
    /// 输入 / 输出 modalities,典型值:`text` / `image` / `audio` / `video` / `pdf`。
    #[serde(default)]
    pub modalities: ModelModalities,
    /// 上下文窗口上限。
    #[serde(default)]
    pub limit: ModelLimit,
    /// "alpha" / "beta" / "deprecated" 标签。
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelModalities {
    #[serde(default)]
    pub input: Vec<String>,
    #[serde(default)]
    pub output: Vec<String>,
}

impl ModelModalities {
    pub fn supports_input(&self, modality: &str) -> bool {
        self.input.iter().any(|m| m.eq_ignore_ascii_case(modality))
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelLimit {
    #[serde(default)]
    pub context: u32,
    #[serde(default)]
    pub output: u32,
}

// ── 进程内单例缓存 ──────────────────────────────────────────────────────────

/// Where the loaded catalog came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogSource {
    /// Downloaded in this session.
    Downloaded,
    /// The disk cache of an earlier download.
    Cache,
    /// The copy built into the binary.
    Bundled,
}

#[derive(Debug, Default)]
struct State {
    /// 已加载的 catalog。`None` 表示从未加载成功。
    catalog: Option<Catalog>,
    /// 缓存最后修改时间(用于判断是否过期)。For the built-in catalog, the time it was taken.
    loaded_at: Option<SystemTime>,
    source: Option<CatalogSource>,
}

fn install(catalog: Catalog, loaded_at: Option<SystemTime>, source: CatalogSource) {
    if let Ok(mut s) = state().write() {
        s.catalog = Some(catalog);
        s.loaded_at = loaded_at;
        s.source = Some(source);
    }
}

fn state() -> &'static RwLock<State> {
    static S: OnceLock<RwLock<State>> = OnceLock::new();
    S.get_or_init(|| RwLock::new(State::default()))
}

fn cache_path() -> PathBuf {
    let mut p = zap_ng_core::paths::cache_dir();
    p.push(CACHE_FILENAME);
    p
}

/// 读已加载的 catalog 副本(无锁等待 — 直接克隆)。
/// 没数据返回 `None`,UI 应展示 "正在拉取" / 重试按钮。
pub fn cached() -> Option<Catalog> {
    state().read().ok().and_then(|s| s.catalog.clone())
}

fn is_loaded() -> bool {
    state().read().is_ok_and(|s| s.catalog.is_some())
}

/// Where the loaded catalog came from and when it was taken, for the settings page.
pub fn loaded_from() -> Option<(CatalogSource, Option<SystemTime>)> {
    let s = state().read().ok()?;
    s.source.map(|source| (source, s.loaded_at))
}

/// The download URL for the `agents.byop.models_dev.url` setting; blank means the default.
pub fn catalog_url(configured: &str) -> &str {
    match configured.trim() {
        "" => DEFAULT_CATALOG_URL,
        url => url,
    }
}

/// Parses a catalog, refusing anything that would leave the page with no providers: a
/// download that returns an error page or an empty object never replaces a real catalog.
fn parse_catalog(bytes: &[u8]) -> Result<Catalog, String> {
    let catalog: Catalog =
        serde_json::from_slice(bytes).map_err(|e| format!("JSON parsing failed: {e}"))?;
    if catalog.is_empty() {
        return Err("the catalog has no providers".to_owned());
    }
    Ok(catalog)
}

fn parse_bundled() -> Result<Catalog, String> {
    let mut json = Vec::new();
    flate2::read::GzDecoder::new(BUNDLED_CATALOG_GZ)
        .read_to_end(&mut json)
        .map_err(|e| format!("the built-in catalog doesn't decompress: {e}"))?;
    parse_catalog(&json)
}

/// When the built-in catalog was taken (`taken_at` in its catalog.txt).
fn bundled_taken_at() -> Option<SystemTime> {
    let value = BUNDLED_CATALOG_INFO
        .lines()
        .find_map(|line| line.strip_prefix("taken_at="))?;
    chrono::DateTime::parse_from_rfc3339(value.trim())
        .ok()
        .map(SystemTime::from)
}

/// Whether the built-in catalog is newer than the disk cache, or there is no cache.
fn prefer_bundled(cache_time: Option<SystemTime>, bundled_time: Option<SystemTime>) -> bool {
    match (cache_time, bundled_time) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(cache), Some(bundled)) => cache < bundled,
    }
}

/// Loads the built-in catalog. Decompressing and parsing it takes a moment, so this runs
/// in a background task.
fn load_bundled() -> bool {
    match parse_bundled() {
        Ok(catalog) => {
            install(catalog, bundled_taken_at(), CatalogSource::Bundled);
            true
        }
        Err(e) => {
            log::error!("[models.dev] {e}");
            false
        }
    }
}

/// Loads the newest catalog this machine has without downloading: the disk cache, or the
/// built-in catalog when there is no cache, the cache doesn't parse, or the build's copy
/// is newer. Returns false only if neither could be loaded. Reads and parses files, so it
/// runs in a background task.
pub fn load_local() -> bool {
    let path = cache_path();
    let cache_time = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
    if !prefer_bundled(cache_time, bundled_taken_at()) && load_cache(&path) {
        return true;
    }
    load_bundled()
}

/// 一个模型从 models.dev 抽出的能力快照,用于 BYOP UI / chat_stream 决策附件类型。
#[derive(Debug, Clone, Default)]
pub struct ModelCaps {
    pub vision: bool,
    pub pdf: bool,
    pub audio: bool,
    pub attachment: bool,
}

impl ModelCaps {
    pub fn from_model(m: &Model) -> Self {
        Self {
            vision: m.modalities.supports_input("image"),
            pdf: m.modalities.supports_input("pdf") || m.attachment,
            audio: m.modalities.supports_input("audio"),
            attachment: m.attachment,
        }
    }
}

/// 在已加载的 catalog 里按 model_id 查找,返回该模型在 models.dev 上声明的能力。
///
/// 优先用 `provider_id` 精确匹配 catalog provider key;miss 时退化到「全 catalog
/// 扫描第一个 model.id 命中」。这样既能精确匹配(用户填的 provider.id 与 models.dev
/// 一致时),又能应对用户自定义 provider id(比如 "openrouter" 或 "siliconflow"
/// 这种聚合 provider 转发上游模型,id 与 models.dev 上游 provider 不同)。
pub fn lookup_caps(provider_id: &str, model_id: &str) -> Option<ModelCaps> {
    let s = state().read().ok()?;
    let catalog = s.catalog.as_ref()?;
    if let Some(p) = catalog.get(provider_id) {
        if let Some(m) = p.models.get(model_id) {
            return Some(ModelCaps::from_model(m));
        }
    }
    for p in catalog.values() {
        if let Some(m) = p.models.get(model_id) {
            return Some(ModelCaps::from_model(m));
        }
    }
    None
}

/// 把磁盘缓存读进内存。
/// 如果磁盘缓存不存在或解析失败,返回 false。
fn load_cache(path: &Path) -> bool {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let mtime = std::fs::metadata(path).ok().and_then(|m| m.modified().ok());
    match parse_catalog(&bytes) {
        Ok(catalog) => {
            install(catalog, mtime, CatalogSource::Cache);
            true
        }
        Err(e) => {
            log::warn!("[models.dev] 解析磁盘缓存失败 ({path:?}): {e}");
            false
        }
    }
}

/// 缓存是否过期 — 不存在或超过 TTL。
fn is_stale_at(loaded_at: Option<SystemTime>, now: SystemTime) -> bool {
    match loaded_at {
        Some(t) => now.duration_since(t).map(|d| d > CACHE_TTL).unwrap_or(true),
        None => true,
    }
}

/// Whether a download is due: never while downloading is off, otherwise when the loaded
/// catalog is missing or older than the TTL.
fn download_due(download_enabled: bool, loaded_at: Option<SystemTime>, now: SystemTime) -> bool {
    download_enabled && is_stale_at(loaded_at, now)
}

/// Whether the settings page should download the catalog now (see [`download_due`]).
pub fn needs_download(download_enabled: bool) -> bool {
    let loaded_at = state().read().ok().and_then(|s| s.loaded_at);
    download_due(download_enabled, loaded_at, SystemTime::now())
}

/// 异步拉取 models.dev 并写入磁盘缓存与内存缓存。
/// 失败仅 log,不向上 propagate(UI 调用方按 `cached()` 是否为 `Some` 决定显示)。
/// The response is checked before it is written, so a bad one leaves the cache alone.
pub async fn fetch_and_cache(client: Client, url: String) -> Result<(), String> {
    let resp = client
        .get(url.as_str())
        .timeout(FETCH_TIMEOUT)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Failed to read the response body: {e}"))?;

    let catalog = parse_catalog(&bytes)?;

    // 写盘 — 失败不算致命,只 log。
    let path = cache_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = std::fs::write(&path, &bytes) {
        log::warn!("[models.dev] 写磁盘缓存失败 ({path:?}): {e}");
    }

    install(catalog, Some(SystemTime::now()), CatalogSource::Downloaded);
    Ok(())
}

/// Opening the settings page: loads the local catalog if none is loaded yet, then
/// downloads a new one when [`needs_download`] says so. Returns whether it downloaded.
pub async fn ensure_loaded(
    client: Client,
    download_enabled: bool,
    url: String,
) -> Result<bool, String> {
    if !is_loaded() {
        load_local();
    }
    if !needs_download(download_enabled) {
        return Ok(false);
    }
    fetch_and_cache(client, url).await.map(|()| true)
}

/// The Refresh button: downloads the catalog regardless of its age, falling back to the
/// local catalog when the download fails and nothing is loaded yet.
pub async fn refresh(client: Client, url: String) -> Result<(), String> {
    let result = fetch_and_cache(client, url).await;
    if result.is_err() && !is_loaded() {
        load_local();
    }
    result
}

// ── chip 行折叠/展开状态(进程级,避免 widget rebuild 丢) ─────────────────

static CHIPS_EXPANDED: AtomicBool = AtomicBool::new(false);

pub fn chips_expanded() -> bool {
    CHIPS_EXPANDED.load(Ordering::Relaxed)
}

pub fn toggle_chips_expanded() {
    CHIPS_EXPANDED.fetch_xor(true, Ordering::Relaxed);
}

// ── 最近一次网络拉取失败标志 ─────────────────────────────────────────────────

static FETCH_FAILED: AtomicBool = AtomicBool::new(false);

/// 最近一次网络拉取是否失败（cached() == None 时有意义）。
pub fn last_fetch_failed() -> bool {
    FETCH_FAILED.load(Ordering::Relaxed)
}

/// 由调用方在 spawn 回调中设置（失败 true，成功不需要重置，因为 cached() 此时为 Some）。
pub fn set_fetch_failed(failed: bool) {
    FETCH_FAILED.store(failed, Ordering::Relaxed);
}

// ── 快速添加 chip 行的搜索过滤 ──────────────────────────────────────────────

fn search_state() -> &'static RwLock<String> {
    static S: OnceLock<RwLock<String>> = OnceLock::new();
    S.get_or_init(|| RwLock::new(String::new()))
}

pub fn search_query() -> String {
    search_state()
        .read()
        .ok()
        .map(|s| s.clone())
        .unwrap_or_default()
}

pub fn set_search_query(q: String) {
    if let Ok(mut s) = search_state().write() {
        *s = q;
    }
}

/// 按当前搜索 query 过滤 catalog,大小写不敏感子串匹配 provider.name 与 provider.id。
/// 空 query 返回全部条目顺序。返回拥有所有权的 Vec 以便 UI 端 take/iter。
pub fn filter_catalog(catalog: &Catalog, query: &str) -> Vec<(String, Provider)> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return catalog
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
    }
    catalog
        .iter()
        .filter(|(id, p)| id.to_lowercase().contains(&q) || p.name.to_lowercase().contains(&q))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// 把 models.dev 的 Model 转换成本地 settings 用的 AgentProviderModel。
///
/// 默认把 catalog 推断的 image/pdf/audio 写进字段(用户首次 sync / quick-add 时
/// 直接看到模型能力被同步进 toml,不需要展开 detail 才看到)。
/// 后续 sync 时调用方只往 None 槽位填新值,Some(_) 视为用户显式覆盖跳过。
pub fn into_agent_provider_model(model: &Model) -> crate::settings::AgentProviderModel {
    let caps = ModelCaps::from_model(model);
    crate::settings::AgentProviderModel {
        name: if model.name.is_empty() {
            model.id.clone()
        } else {
            model.name.clone()
        },
        id: model.id.clone(),
        context_window: model.limit.context,
        max_output_tokens: model.limit.output,
        reasoning: model.reasoning,
        tool_call: model.tool_call,
        image: Some(caps.vision),
        pdf: Some(caps.pdf),
        audio: Some(caps.audio),
        enabled: true,
    }
}

#[cfg(test)]
#[path = "models_dev_tests.rs"]
mod tests;
