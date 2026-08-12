//! HTTP 客户端。
//!
//! TLS 走 rustls + **平台信任库**（`platform-verifier`），不是编译期内置根证书：
//! 本工具的目标用户全都开着代理，其中做 TLS 中间人的那类会让内置根证书下的所有探测直接失败。

use std::time::Duration;

/// 建一个带全局超时的 agent。所有探测共用一个——连接池复用，且超时策略只有一处。
pub fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        // 明确一个 UA：匿名 UA 更容易被第三方的风控挡掉，且出问题时对方能认出是谁。
        .user_agent(concat!("ipcheck/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

/// GET 并读成字符串。任何失败都塌缩成 `None`——探测层只关心"拿到了没有"，
/// 具体是 DNS 失败还是 502 不改变结论，也不该把第三方的错误文本透给用户。
pub fn get_text(agent: &ureq::Agent, url: &str) -> Option<String> {
    let mut response = agent.get(url).call().ok()?;
    if response.status() != 200 {
        return None;
    }
    response.body_mut().read_to_string().ok()
}
