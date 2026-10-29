//! What the plugin does, without Pumpkin: which chat messages are questions for Kira, what it
//! sends to Kira's endpoint, what it prints back, and its config file.
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// How long a question may wait for Kira's answer, connection included.
pub const TIMEOUT: Duration = Duration::from_secs(10);
/// Seconds a player waits between two questions (Kira has her own limits too).
pub const COOLDOWN: Duration = Duration::from_secs(10);
/// At most this many characters of an answer reach chat.
pub const MAX_ANSWER: usize = 256;
/// What the chat box lets a player type, and what the endpoint accepts.
pub const MAX_QUESTION: usize = 256;

/// The question in a chat message that starts with `kira` or `@kira` (any case), or `None`.
/// "kira why can't I break this" → "why can't I break this"; "kiralol", "hey kira" → `None`.
pub fn question(message: &str) -> Option<&str> {
    let text = message.trim_start();
    let text = text.strip_prefix('@').unwrap_or(text);
    let head = text.get(..4)?;
    if !head.eq_ignore_ascii_case("kira") {
        return None;
    }
    let rest = &text[4..];
    if rest
        .chars()
        .next()
        .is_some_and(|c| c.is_alphanumeric() || c == '_')
    {
        return None;
    }
    let q = rest
        .trim_start_matches(|c: char| c.is_whitespace() || ",:;!?.-".contains(c))
        .trim_end();
    (!q.is_empty()).then_some(q)
}

#[derive(Serialize)]
struct Request<'a> {
    player: &'a str,
    question: &'a str,
}

/// The JSON body for `POST /game/ask`. The server is named by its token, not the body.
pub fn request_body(player: &str, message: &str) -> String {
    let question: String = message.chars().take(MAX_QUESTION).collect();
    serde_json::to_string(&Request {
        player,
        question: &question,
    })
    .expect("two strings always serialize")
}

#[derive(Deserialize)]
struct Response {
    #[serde(default)]
    answer: String,
}

/// The chat line for Kira's answer (`<Kira> …`), or `None` when she has nothing to say (an empty
/// answer, a body that isn't hers). Formatting codes and control characters never reach chat.
pub fn chat_line(body: &[u8]) -> Option<String> {
    let answer = serde_json::from_slice::<Response>(body).ok()?.answer;
    let clean: String = answer
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .filter(|&c| c != '§')
        .take(MAX_ANSWER)
        .collect();
    let clean = clean.split_whitespace().collect::<Vec<_>>().join(" ");
    (!clean.is_empty()).then(|| format!("<Kira> {clean}"))
}

/// `plugins/data/kira-pumpkin/config.toml`.
#[derive(Debug, Deserialize, PartialEq)]
pub struct Config {
    /// Kira's endpoint, `https://…/game/ask`.
    pub endpoint: String,
    /// This server's token, from Horizon's team (`kira_…`).
    pub token: String,
}

impl Config {
    pub fn parse(text: &str) -> Result<Config, String> {
        let config: Config = toml::from_str(text).map_err(|e| e.to_string())?;
        if !config.endpoint.starts_with("https://") {
            return Err("endpoint must be an https:// address".into());
        }
        if !config.token.starts_with("kira_") {
            return Err("token must be the kira_… token Horizon's team sent".into());
        }
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_question_is_kira_or_at_kira_at_the_start_in_any_case() {
        assert_eq!(
            question("kira why can't I break this block"),
            Some("why can't I break this block")
        );
        assert_eq!(
            question("@Kira, how do I sign in?"),
            Some("how do I sign in?")
        );
        assert_eq!(question("KIRA: hi"), Some("hi"));
        assert_eq!(
            question("  kira   where is spawn  "),
            Some("where is spawn")
        );
        for no in [
            "kira",
            "@kira  ",
            "kiralol hi",
            "kira_x hi",
            "hey kira why",
            "",
            "kir",
            "ki☃a hi",
        ] {
            assert_eq!(question(no), None, "{no}");
        }
    }

    #[test]
    fn the_request_names_the_player_and_the_question() {
        let body = request_body("Steve_1", "kira why \"this\"");
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(
            v,
            serde_json::json!({ "player": "Steve_1", "question": "kira why \"this\"" })
        );
        let long = request_body("Steve", &"é".repeat(400));
        let v: serde_json::Value = serde_json::from_str(&long).unwrap();
        assert_eq!(
            v["question"].as_str().unwrap().chars().count(),
            MAX_QUESTION
        );
    }

    #[test]
    fn the_answer_becomes_one_clean_chat_line() {
        assert_eq!(
            chat_line(br#"{"answer":"Dude. Hold left click."}"#).as_deref(),
            Some("<Kira> Dude. Hold left click.")
        );
        assert_eq!(
            chat_line("{\"answer\":\"§cred\\nnext  line\"}".as_bytes()).as_deref(),
            Some("<Kira> cred next line")
        );
        assert_eq!(chat_line(br#"{"answer":""}"#), None);
        assert_eq!(chat_line(br#"{"answer":"   "}"#), None);
        assert_eq!(chat_line(br#"{"error":"token unknown"}"#), None);
        assert_eq!(chat_line(b"<html>bad gateway</html>"), None);
        let long = format!(r#"{{"answer":"{}"}}"#, "a".repeat(400));
        assert_eq!(
            chat_line(long.as_bytes()).unwrap().len(),
            "<Kira> ".len() + MAX_ANSWER
        );
    }

    #[test]
    fn the_config_needs_an_https_endpoint_and_a_kira_token() {
        let ok =
            "endpoint = \"https://mcp.playhorizon.dev/game/ask\"\ntoken = \"kira_0123abcd_x\"\n";
        assert_eq!(
            Config::parse(ok).unwrap(),
            Config {
                endpoint: "https://mcp.playhorizon.dev/game/ask".into(),
                token: "kira_0123abcd_x".into()
            }
        );
        assert!(Config::parse("endpoint = \"http://x/game/ask\"\ntoken = \"kira_x\"").is_err());
        assert!(Config::parse("endpoint = \"https://x/game/ask\"\ntoken = \"\"").is_err());
        assert!(Config::parse("endpoint = \"https://x/game/ask\"").is_err());
    }
}
