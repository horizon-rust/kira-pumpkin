# kira-pumpkin

Kira, the bot of [Horizon](https://playhorizon.dev)'s Discord server, answering in a [Pumpkin](https://pumpkinmc.org)
server's chat. A player writes a message that starts with `kira` or `@kira` (any case):

```
<Steve> kira why can't I break this block
<Kira> Dude, that's bedrock. Nobody breaks bedrock in survival.
```

Kira answers from Horizon's FAQ, docs and release notes, with her Discord rules: only what works in a release, no
dates or promises. At most 256 characters, plain text. The player's own message is never held back or changed.

## Install

1. Download `kira_pumpkin.wasm` from the [latest release](https://github.com/horizon-rust/kira-pumpkin/releases/latest)
   and put it in your server's `plugins/` folder.
2. Ask the Horizon Team for a token for your server. Kira sends it to you by Discord DM, never in a channel.
3. Create `plugins/data/kira-pumpkin/config.toml`:

   ```toml
   endpoint = "https://mcp.playhorizon.dev/game/ask"
   token = "kira_…"
   ```

4. Restart the server. The log says `kira-pumpkin: Kira answers chat messages that start with kira or @kira`.

The plugin asks Pumpkin for three permissions: `HTTP_OUTBOUND` (to reach Kira), `NETWORK_DNS` (to resolve her
address) and `FS_READ_DATA` (to read `config.toml`). If your server restricts plugin permissions, allow these.

Keep the token secret: anyone with it can ask Kira in your server's name. A leaked token is revoked by the Horizon
Team, who send you a new one.

## How it works

- A non-blocking handler of Pumpkin's async chat event: it runs after the blocking handlers and never cancels or
  edits the message.
- For a message that starts with `kira`/`@kira`, it POSTs `{ "player", "question" }` to the endpoint with the
  token. The connection, the first byte and each later byte each have a 10-second timeout (`wasi:http`). Then it
  broadcasts `<Kira> …` to the server.
- One question per player every 10 seconds on the plugin's side. Kira also limits each player (3 questions in 10
  minutes) and each server (30 an hour, 200 a day), and has a daily budget. When a limit is reached, she says so
  once or stays quiet.
- Errors (no answer in time, a revoked token) go to the server log. Nothing is printed in chat.

## Build

```sh
rustup target add wasm32-wasip2
cargo build --release --target wasm32-wasip2   # target/wasm32-wasip2/release/kira_pumpkin.wasm
cargo test                                     # the chat matching, the request, the answer, the config
```

`examples/ask.rs` sends one question through the plugin's own HTTP client outside Pumpkin, with
[wasmtime](https://wasmtime.dev) (instructions at the top of the file).

Built against `pumpkin-plugin-api` 0.2.1 (Minecraft 26.3). It is a WebAssembly component, so one file works on
every OS and architecture Pumpkin runs on.

## Licence

MIT or Apache-2.0, at your choice.
