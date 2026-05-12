use crate::types::{ClientId, RateConfig, Username};
use worker::kv::KvStore;

const GLOBAL_USERNAME_MAX: u32 = 60;
const GLOBAL_IP_MAX: u32 = 100;

pub struct Limiter<'a> {
    kv: &'a KvStore,
}

pub struct PendingWrites {
    pub key: String,
    pub count_value: String,
    pub count_ttl: u64,
    pub block_key: Option<String>,
    pub block_ttl: Option<u64>,
}

pub struct GlobalCheckResult {
    pub global_ip_blocked: bool,
    pub username_blocked: bool,
    pub global_ip_write: Option<PendingWrite>,
    pub username_write: Option<PendingWrite>,
}

pub struct PendingWrite {
    pub key: String,
    pub value: String,
    pub ttl: u64,
}

impl<'a> Limiter<'a> {
    pub fn new(kv: &'a KvStore) -> Self {
        Self { kv }
    }

    pub async fn check_globals(
        &self,
        client: &ClientId,
        user: &Username,
    ) -> Result<GlobalCheckResult, worker::Error> {
        let now = worker::Date::now().as_millis() / 1000;
        let win_secs = 60u64;
        let win = (now / win_secs) * win_secs;
        let ttl = win_secs.saturating_sub(now - win).max(60);

        let global_ip_key = format!("rl:gl:{}:{}", client.0, win);
        let username_key = format!("rl:ugl:{}:{}", user.as_str(), win);

        // Both KV reads fire concurrently.
        let (global_ip_res, username_res) = futures::join!(
            self.kv.get(&global_ip_key).text(),
            self.kv.get(&username_key).text(),
        );

        let global_ip_cnt: u32 = global_ip_res?
            .as_deref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let global_ip_blocked = global_ip_cnt >= GLOBAL_IP_MAX;
        if global_ip_blocked {
            worker::console_log!("Global IP limit: {}", client.0);
        }
        let new_global_ip = global_ip_cnt.saturating_add(1);
        let global_ip_write = Some(PendingWrite {
            key: global_ip_key,
            value: new_global_ip.to_string(),
            ttl,
        });

        let username_cnt: u32 = username_res?
            .as_deref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let username_blocked = username_cnt >= GLOBAL_USERNAME_MAX;
        if username_blocked {
            worker::console_log!("Global username limit: {}", user.as_str());
        }
        let new_username = username_cnt.saturating_add(1);
        let username_write = Some(PendingWrite {
            key: username_key,
            value: new_username.to_string(),
            ttl,
        });

        Ok(GlobalCheckResult {
            global_ip_blocked,
            username_blocked,
            global_ip_write,
            username_write,
        })
    }

    pub async fn check_with_info(
        &self,
        client: &ClientId,
        user: &Username,
        cfg: RateConfig,
    ) -> Result<Option<((u32, u64), PendingWrites)>, worker::Error> {
        let now = worker::Date::now().as_millis() / 1000;
        let win = (now / cfg.window_secs) * cfg.window_secs;

        let key = format!("rl:{}:{}:{}", client.0, user.as_str(), win);
        let block_key = format!("block:{}:{}", client.0, user.as_str());

        let (block_res, cnt_res) = futures::join!(
            self.kv.get(&block_key).text(),
            self.kv.get(&key).text(),
        );

        if block_res?.is_some() {
            return Ok(None);
        }

        let cnt: u32 = cnt_res?
            .as_deref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        if cnt >= cfg.max_req {
            worker::console_log!("Blocked: {} for {}", client.0, user.as_str());
            let writes = PendingWrites {
                key,
                count_value: cnt.to_string(),
                count_ttl: cfg.window_secs,
                block_key: Some(block_key),
                block_ttl: Some(cfg.block_secs),
            };
            return Ok(Some(((0, cfg.block_secs), writes)));
        }

        let new_cnt = cnt.saturating_add(1);
        let ttl = cfg.window_secs.saturating_sub(now - win).max(60);

        if new_cnt >= (cfg.max_req as f32 * 0.8) as u32 {
            worker::console_log!(
                "Warn: {}/{} for {}",
                new_cnt,
                cfg.max_req,
                client.0
            );
        }

        let remaining = cfg.max_req.saturating_sub(new_cnt);
        let reset = win + cfg.window_secs;

        let writes = PendingWrites {
            key,
            count_value: new_cnt.to_string(),
            count_ttl: ttl,
            block_key: None,
            block_ttl: None,
        };

        Ok(Some(((remaining, reset), writes)))
    }
}

pub async fn flush_global_writes(kv: &KvStore, writes: GlobalCheckResult) {
    if let Some(w) = writes.global_ip_write {
        if let Ok(b) = kv.put(&w.key, &w.value) {
            let _ = b.expiration_ttl(w.ttl).execute().await;
        }
    }
    if let Some(w) = writes.username_write {
        if let Ok(b) = kv.put(&w.key, &w.value) {
            let _ = b.expiration_ttl(w.ttl).execute().await;
        }
    }
}

pub async fn flush_rate_writes(kv: &KvStore, writes: PendingWrites) {
    if let Some(block_key) = writes.block_key {
        if let Some(block_ttl) = writes.block_ttl {
            if let Ok(b) = kv.put(&block_key, "blocked") {
                let _ = b.expiration_ttl(block_ttl).execute().await;
            }
        }
    } else {
        if let Ok(b) = kv.put(&writes.key, &writes.count_value) {
            let _ = b.expiration_ttl(writes.count_ttl).execute().await;
        }
    }
}
