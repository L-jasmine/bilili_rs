# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`bilili_rs` is a Rust SDK for interacting with Bilibili's live streaming platform. It provides:
- WebSocket connection to Bilibili live chat rooms for receiving real-time messages (danmu, gifts, etc.)
- HTTP API client for interacting with Bilibili services (sending barrages, gifts, likes)
- QR code-based login flow for authentication

## Development Commands

### Building
```bash
cargo build
cargo build --release
```

### Testing
Tests require a `token` file in the project root containing Bilibili session cookies (one per line). This file is gitignored.

```bash
# Run all tests
cargo test

# Run a specific test (examples from inline doc comments)
cargo test --package bilili_rs --lib -- api::test_share_room --exact --nocapture
cargo test --package bilili_rs --lib -- api::test_send_gift --exact --nocapture
```

### Publishing
```bash
cargo package
cargo publish
```

## Architecture

The crate is organized into two main modules:

### `api` - HTTP API Client
- **`APIClient`**: Main HTTP client with cookie management for authenticated requests
- **`UserToken`**: Parses and stores Bilibili cookies (DedeUserID, SESSDATA, bili_jct)
- **`LoginUrl` / `LoginManager`**: QR code login flow - generates QR, polls for scan confirmation
- **WBI signing** (`api/wbi.rs`): Bilibili's request signing mechanism using cached mixin keys

Key API operations:
- `send_barrage()` - Send chat messages to live room
- `send_gift()` - Send gifts using gold coins
- `like_report_v3()` - Like a live room
- `share_room()` - Share a live room
- `get_danmu_info()` - Get WebSocket server info for live chat connection
- `get_room_play_info()` - Get live room status

### `live_ws` - WebSocket Live Chat
- **`connect()`**: Entry point - creates `MsgStream` for receiving live messages
- **`open_client()`**: Core connection loop with auto-reconnect (exponential backoff: 10s → 300s)
- **`connect_keep()`**: Sends heartbeat every 30 seconds
- **`loop_handle_msg()`**: Receives and decodes binary WebSocket protocol messages

The WebSocket protocol uses a binary format with:
- Package header: length (4B) + header_len (2B) + version (2B) + type (4B) + other (4B)
- Version 2 = zlib compressed body (auto-inflates and re-parses)
- Version 1 = raw JSON body
- Message types: 3=heartbeat, 5=notification, 8=login ack

### `live_ws/message` - Message Types
- **`ServerLiveMessage`**: Enum of messages from server (LoginAck, Notification, ServerHeartBeat)
- **`ClientLiveMessage`**: Messages to server (Login, ClientHeartBeat)
- **`NotificationMsg`**: Tagged enum of all chat events (DANMU_MSG, SEND_GIFT, INTERACT_WORD, etc.)
  - Uses serde's `tag = "cmd"` for dispatching on the `cmd` field
  - Contains custom deserialization logic for `DanmuMsg` (array-based format)
  - Some variants have `#[cfg(debug_assertions)]` extra fields for capturing unknown data

## Important Implementation Details

1. **Cookie Storage**: The `token` file (gitignored) stores session cookies for testing. Format is one Set-Cookie header value per line.

2. **Reconnection Logic**: WebSocket client auto-reconnects with increasing delays. Resets retry count after 30 minutes of stable connection.

3. **WBI Signing**: Required for many API calls. Keys are cached for 5 hours (`#[cached(time = 18000)]`). The signature uses a specific character shuffling table defined in `wbi.rs`.

4. **User-Agent**: Uses a specific Chrome UA string for API compatibility.

5. **Test Isolation**: Integration tests read from the `token` file for credentials. Ensure this file exists before running tests.

## Dependencies
- `reqwest` - HTTP client with cookie support
- `tokio-tungstenite` - WebSocket client
- `serde`/`serde_json` - JSON serialization
- `inflate` - zlib decompression for WebSocket protocol v2
- `cached` - WBI key caching
- `thiserror` - Error types
