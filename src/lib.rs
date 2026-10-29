//! Kira in a Pumpkin server's chat: a player writes `kira …` or `@kira …`, the plugin asks Kira's
//! endpoint (`POST /game/ask` on Kira's host) and prints her answer in chat as `<Kira> …`.
//!
//! [`ask`] is the plugin's logic without Pumpkin (what is a question, the request, the answer, the
//! config), tested on the host. The plugin and its HTTP client only build for `wasm32-wasip2`.
pub mod ask;

#[cfg(target_family = "wasm")]
mod http;
#[cfg(all(target_family = "wasm", feature = "plugin"))]
mod plugin;
#[cfg(target_family = "wasm")]
pub use http::post_json;
