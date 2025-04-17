Got it! Here's a revised `README.md` tailored to your project structure, with `lib.rs` as the entry point and `cal_jambonz` noted as an internal crate:

---

# 🛠️ Jambonz WebSocket Server

A modular WebSocket server framework built on [`actix-web`](https://crates.io/crates/actix-web) and [`actix-ws`](https://crates.io/crates/actix-ws) designed for real-time voice and messaging applications using [Jambonz](https://www.jambonz.org/). This server provides a clean abstraction over Jambonz WebSocket message handling and lets you plug in your business logic using simple async handlers.

---

## 📁 Project Structure

```
src/
├── handler.rs           # WebSocket message handler logic
└── lib.rs               # Server definition, route registration, and startup logic
```

---

## 🧩 Features

- ✅ WebSocket route registration with custom paths
- ✅ Supports multiple Jambonz message types (`Hook`, `Recording`)
- ✅ Built-in support for protocol-specific headers
- ✅ Async handler registration with shared application state
- ✅ Based on `actix-web` and `actix-ws`
- ✅ Clean abstraction via `HandlerContext<T>`

---

## 📦 Dependencies

- [`actix-web`](https://crates.io/crates/actix-web)
- [`actix-ws`](https://crates.io/crates/actix-ws)
- [`uuid`](https://crates.io/crates/uuid)
- [`futures`](https://crates.io/crates/futures)
- `cal_jambonz` – Internal crate for handling Jambonz WebSocket payloads

---

## 🚀 Getting Started

### 1. Define Your App State

```rust
#[derive(Clone)]
pub struct AppState {
    pub message: String,
}
```

### 2. Register a WebSocket Handler

```rust
let hook_handler = register_handler(|mut ctx: HandlerContext<AppState>| async move {
    if let JambonzRequest::TextMessage(WebsocketRequest::SessionNew(new_session)) = ctx.request {
        let ack = Verbs::new(&new_session.msgid)
            .say(Say::new("Welcome! Please wait while we find an agent.".to_string()))
            .as_ack_reply()
            .json();

        let _ = ctx.session.text(ack).await;
    }
});
```

### 3. Configure and Start the Server

```rust
let route = JambonzRoute {
    path: "/hook".to_string(),
    ws_type: JambonzRouteType::Hook,
    handler: hook_handler,
};

JambonzWebServer::new(AppState { message: "Hello".into() })
    .with_bind_ip("127.0.0.1")
    .with_bind_port(8080)
    .add_route(route)
    .start()
    .await
    .expect("Failed to start server");
```

---

## 🧱 Core Types

### `JambonzWebServer<T>`
Main server structure used to configure and start your WebSocket server.

- `with_bind_ip(ip: &str)`: Set the bind IP address
- `with_bind_port(port: u16)`: Set the listening port
- `add_route(route: JambonzRoute<T>)`: Register a WebSocket route
- `start()`: Launch the server

### `JambonzRoute<T>`
Defines a WebSocket route.

- `path`: The route path (e.g., `"/hook"`)
- `ws_type`: Protocol type (`Hook` or `Recording`)
- `handler`: Your async handler registered via `register_handler`

### `HandlerContext<T>`
Passed to every handler. Includes:

- `uuid`: Request ID for tracing
- `session`: WebSocket session object
- `request`: Parsed `JambonzRequest`
- `state`: Your shared application state (`actix_web::web::Data<T>`)

---

## 📘 Internal Crate: `cal_jambonz`

This project depends on `cal_jambonz`, your internal crate that defines the structure for handling Jambonz messages, including `JambonzRequest` and `WebsocketRequest`.

---

## 🛡️ License

MIT – Use it freely in your own Jambonz-based projects. Contributions welcome!

---
