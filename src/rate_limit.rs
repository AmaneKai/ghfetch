use crate::types::{ClientId, RateConfig, Username};
use worker::kv::KvStore;

// Maximum unique usernames queried across ALL IPs per minute
const GLOBAL_USERNAME_MAX: u32 = 60;
// Maximum requests per IP per minute globally
const GLOBAL_IP_MAX: u32 = 100;

pub struct Limiter<'a> {
    kv: &'a KvStore,
}

impl<'a> Limiter<'a> {
    pub fn new(kv: &'a KvStore) -> Self {
        Self { kv }
    }

    // Returns Some((remaining, reset_secs)) if allowed, None if blocked
    pub async fn check_with_info(
        &self,
        client: &ClientId,
        user: &Username,
        cfg: RateConfig,
    ) -> Result<Option<(u32, u64)>, worker::Error> {
        let now = worker::Date::now().as_millis() / 1000;
        let win = (now / cfg.window_secs) * cfg.window_secs;

        let key = format!("rl:{}:{}:{}", client.0, user.as_str(), win);
        let block = format!("block:{}:{}", client.0, user.as_str());

        if self.kv.get(&block).text().await?.is_some() {
            return Ok(None);
        }

        let cnt_str = self.kv.get(&key).text().await?;
        let cnt: u32 = cnt_str
            .as_deref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        if cnt >= cfg.max_req {
            self.kv
                .put(&block, "blocked")?
                .expiration_ttl(cfg.block_secs)
                .execute()
                .await?;
            worker::console_log!("Blocked: {} for {}", client.0, user.as_str());
            return Ok(None);
        }

        let new_cnt = cnt.saturating_add(1);
        let ttl = cfg.window_secs.saturating_sub(now - win).max(60);

        self.kv
            .put(&key, &new_cnt.to_string())?
            .expiration_ttl(ttl)
            .execute()
            .await?;

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
        Ok(Some((remaining, reset)))
    }

    pub async fn check_global(&self, client: &ClientId) -> Result<bool, worker::Error> {
        let now = worker::Date::now().as_millis() / 1000;
        let win_secs = 60u64;
        let win = (now / win_secs) * win_secs;
        let key = format!("rl:gl:{}:{}", client.0, win);

        let cnt_str = self.kv.get(&key).text().await?;
        let cnt: u32 = cnt_str
            .as_deref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        if cnt >= GLOBAL_IP_MAX {
            worker::console_log!("Global IP limit: {}", client.0);
            return Ok(true);
        }

        let new_cnt = cnt.saturating_add(1);
        let ttl = win_secs.saturating_sub(now - win).max(60);

        self.kv
            .put(&key, &new_cnt.to_string())?
            .expiration_ttl(ttl)
            .execute()
            .await?;

        Ok(false)
    }

    // Prevents enumeration attacks where many IPs each query unique usernames,
    // bypassing per-IP limits and burning the GitHub token.
    pub async fn check_username_global(
        &self,
        user: &Username,
    ) -> Result<bool, worker::Error> {
        let now = worker::Date::now().as_millis() / 1000;
        let win_secs = 60u64;
        let win = (now / win_secs) * win_secs;
        let key = format!("rl:ugl:{}:{}", user.as_str(), win);

        let cnt_str = self.kv.get(&key).text().await?;
        let cnt: u32 = cnt_str
            .as_deref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        if cnt >= GLOBAL_USERNAME_MAX {
            worker::console_log!("Global username limit: {}", user.as_str());
            return Ok(true);
        }

        let new_cnt = cnt.saturating_add(1);
        let ttl = win_secs.saturating_sub(now - win).max(60);

        self.kv
            .put(&key, &new_cnt.to_string())?
            .expiration_ttl(ttl)
            .execute()
            .await?;

        Ok(false)
    }
}
