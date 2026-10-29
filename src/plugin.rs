//! The Pumpkin side: a non-blocking handler of the async chat event (the player's message is never
//! held back or changed), which asks Kira and broadcasts her answer.
use crate::ask::{self, Config};
use crate::http::post_json;
use pumpkin_plugin_api::events::{AsyncPlayerChatEvent, EventData, EventHandler, EventPriority};
use pumpkin_plugin_api::permissions::{FS_READ_DATA, HTTP_OUTBOUND, NETWORK_DNS};
use pumpkin_plugin_api::{Context, Plugin, PluginMetadata, Server};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;
use tracing::{info, warn};

struct KiraPlugin;

struct Chat {
    config: Config,
    /// When each player (lowercase name) last asked.
    asked: Mutex<HashMap<String, Instant>>,
}

impl Chat {
    /// False when `player` asked less than [`ask::COOLDOWN`] ago; else notes the question.
    fn may_ask(&self, player: &str) -> bool {
        let now = Instant::now();
        let mut asked = self.asked.lock().unwrap_or_else(|e| e.into_inner());
        asked.retain(|_, at| now.duration_since(*at) < ask::COOLDOWN);
        if asked.contains_key(player) {
            return false;
        }
        asked.insert(player.to_owned(), now);
        true
    }
}

impl EventHandler<AsyncPlayerChatEvent> for Chat {
    fn handle(
        &self,
        server: Server,
        event: EventData<AsyncPlayerChatEvent>,
    ) -> EventData<AsyncPlayerChatEvent> {
        if event.cancelled || ask::question(&event.message).is_none() {
            return event;
        }
        let player = event.player.get_name();
        if !self.may_ask(&player.to_ascii_lowercase()) {
            return event;
        }
        let body = ask::request_body(&player, &event.message);
        match post_json(
            &self.config.endpoint,
            &self.config.token,
            &body,
            ask::TIMEOUT,
        ) {
            Ok((_, reply)) => {
                if let Some(line) = ask::chat_line(&reply) {
                    server.broadcast(&line);
                }
            }
            Err(e) => warn!("kira-pumpkin: no answer for {player}: {e}"),
        }
        event
    }
}

impl Plugin for KiraPlugin {
    fn new() -> Self {
        KiraPlugin
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "kira-pumpkin".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            authors: vec!["Horizon".into()],
            description: "Kira, the bot of Horizon's Discord server, answers in chat: start a message with kira or @kira.".into(),
            dependencies: vec![],
            permissions: vec![HTTP_OUTBOUND.into(), NETWORK_DNS.into(), FS_READ_DATA.into()],
        }
    }

    fn on_load(&self, context: Context) -> pumpkin_plugin_api::Result<()> {
        let path = format!("{}/config.toml", context.get_data_folder());
        let text = std::fs::read_to_string(&path).map_err(|e| {
            format!("kira-pumpkin: read {path}: {e} (see the README: endpoint and token)")
        })?;
        let config = Config::parse(&text).map_err(|e| format!("kira-pumpkin: {path}: {e}"))?;
        context.register_event_handler(
            Chat {
                config,
                asked: Mutex::new(HashMap::new()),
            },
            EventPriority::Normal,
            false,
        )?;
        info!("kira-pumpkin: Kira answers chat messages that start with kira or @kira");
        Ok(())
    }

    fn on_unload(&self, _context: Context) -> pumpkin_plugin_api::Result<()> {
        Ok(())
    }
}

pumpkin_plugin_api::register_plugin!(KiraPlugin);
