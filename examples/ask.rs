//! One question to Kira's endpoint through the plugin's own HTTP client, outside Pumpkin:
//!   cargo build --release --target wasm32-wasip2 --no-default-features --example ask
//!   KIRA_TOKEN=… wasmtime run -S http --env KIRA_TOKEN --env KIRA_ENDPOINT=https://…/game/ask \
//!     target/wasm32-wasip2/release/examples/ask.wasm Steve "kira why can't I break this block"
#[cfg(target_family = "wasm")]
fn main() {
    use kira_pumpkin::ask;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [player, message] = &args[..] else {
        eprintln!("usage: ask <player> <message>");
        std::process::exit(2);
    };
    let (endpoint, token) = (
        std::env::var("KIRA_ENDPOINT").unwrap(),
        std::env::var("KIRA_TOKEN").unwrap(),
    );
    if ask::question(message).is_none() {
        println!("(not a question for Kira)");
        return;
    }
    match kira_pumpkin::post_json(
        &endpoint,
        &token,
        &ask::request_body(player, message),
        ask::TIMEOUT,
    ) {
        Ok((status, body)) => println!(
            "HTTP {status}: {}",
            ask::chat_line(&body).unwrap_or_else(|| "(nothing in chat)".into())
        ),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    eprintln!("build for wasm32-wasip2 and run it with wasmtime (see the top of this file)");
}
