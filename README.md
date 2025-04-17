# 🛰️ Jambonz WebSocket Server Framework

A lightweight, extensible WebSocket framework built with [Actix Web](https://actix.rs/) for building voice and media applications using the [Jambonz](https://www.jambonz.org/) protocol. It allows dynamic routing and state-aware handling of WebSocket messages, such as call hooks and audio recordings.

---

## ✨ Features

- 🔧 Register Jambonz-style WebSocket routes (e.g., hooks, recordings)
- 🧠 Share application state across handlers
- 📡 Auto-negotiated subprotocol headers (`ws.jambonz.org`, `audio.jambonz.org`)
- 🚀 Async-first with Actix runtime and WebSocket support
- ⚙️ Minimal setup and extensible architecture

---

## 🔧 Quick Start

### 1. Define Your App State

```rust
#[derive(Clone)]
pub struct AppState {
    pub message: String,
}
```

---

### 2. Define Your Handlers

#### 🎙️ Recording Handler

```rust
use cal_jambonz::ws::{JambonzRequest, RecordingRequest};

let recording_handler = register_handler(|mut ctx: HandlerContext<AppState>| async move {
    match ctx.request {
        JambonzRequest::Recording(RecordingRequest::SessionNew(session)) => {
            println!("Started recording session: {:?}", session);
        }
        JambonzRequest::Recording(RecordingRequest::Binary(bytes)) => {
            println!("Received audio bytes: {} bytes", bytes.len());
        }
        JambonzRequest::Recording(RecordingRequest::Close) => {
            println!("Recording session closed");
        }
        _ => {}
    }
});
```

#### 🔁 Hook Handler (Expanded Match)

```rust
use cal_jambonz::verbs::{Verbs, Say};
use cal_jambonz::ws::{JambonzRequest, WebsocketRequest};

let hook_handler = register_handler(|mut ctx: HandlerContext<AppState>| async move {
    match ctx.request {
        JambonzRequest::Hook(req) => {
            match req {
                WebsocketRequest::SessionNew(new_session) => {
                    let ack = Verbs::new(&new_session.msgid)
                        .say(Say::new("Welcome! Please wait while we find an agent.".to_string()))
                        .as_ack_reply()
                        .json();

                    let _ = ctx.session.text(ack).await;
                }

                WebsocketRequest::SessionRedirect(redirect) => {
                    println!("Session redirected: {:?}", redirect);
                }

                WebsocketRequest::SessionReconnect(reconnect) => {
                    println!("Session reconnected: {:?}", reconnect);
                }

                WebsocketRequest::CallStatus(status) => {
                    println!("Call status update: {:?}", status);
                }

                WebsocketRequest::VerbHook(hook) => {
                    println!("Verb hook event: {:?}", hook);
                }

                WebsocketRequest::Close => {
                    println!("Session closed");
                }
            }
        }

        _ => {
            println!("Received non-hook request on hook handler");
        }
    }
});
```

---

### 3. Register Routes and Start Server

```rust
use your_crate::{JambonzRoute, JambonzRouteType, JambonzWebServer};

let server = JambonzWebServer::new(AppState {
    message: "stateful message".into(),
})
.with_bind_ip("127.0.0.1")
.with_bind_port(3000)
.add_route(JambonzRoute {
    path: "/hook".into(),
    ws_type: JambonzRouteType::Hook,
    handler: hook_handler,
})
.add_route(JambonzRoute {
    path: "/recording".into(),
    ws_type: JambonzRouteType::Recording,
    handler: recording_handler,
});

server.start();
```

---

## 📦 JambonzRequest Explained

The core WebSocket message types are represented using the `JambonzRequest` enum:

```rust
pub enum JambonzRequest {
    Hook(WebsocketRequest),
    Recording(RecordingRequest),
}
```

### 🔁 Hook Events

```rust
pub enum WebsocketRequest {
    SessionNew(SessionNew),
    SessionRedirect(SessionRedirect),
    SessionReconnect(SessionReconnect),
    CallStatus(SessionCallStatus),
    VerbHook(SessionVerbHook),
    Close,
}
```

These represent various control and lifecycle events during a live call.

---

### 🎙️ Recording Events

```rust
pub enum RecordingRequest {
    SessionNew(SessionRecording),
    Binary(Vec<u8>),
    Close,
}
```

Used for handling real-time audio streams and recording metadata.

---

## 🧰 Handler Context

Each handler receives a `HandlerContext<T>` containing:

```rust
pub struct HandlerContext<T> {
    pub uuid: Uuid,                // Unique per-session ID
    pub session: Session,          // WebSocket session object
    pub request: JambonzRequest,   // Incoming parsed request
    pub state: Data<T>,            // Your app state
}
```

Use `ctx.session.text(...)` to respond directly to the WebSocket client.

---

## ✅ Requirements

- Rust 1.70+
- Actix Web ecosystem
- Features from: `actix-web`, `actix-ws`, `uuid`, `serde`, `futures`, etc.

---

## 🧱 Contributing

PRs and issues are welcome. Whether it’s bug fixes, new features, or documentation improvements — let’s build together.

---

## 📄 License

MIT