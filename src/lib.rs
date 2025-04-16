mod handler;

use actix_web::dev::Server;
use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::web::{Data, Payload};
use actix_web::{App, Error, HttpRequest, HttpResponse, HttpServer, rt, web};
use actix_ws::Session;
use cal_jambonz::ws::WebsocketRequest;
use std::pin::Pin;
use std::sync::Arc;
use uuid::Uuid;

pub type HandlerFn<T> =
    Arc<dyn Fn(HandlerContext<T>) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

pub fn register_handler<T, F, Fut>(handler: F) -> HandlerFn<T>
where
    T: Clone + Send + Sync + 'static,
    F: Fn(HandlerContext<T>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    Arc::new(move |ctx: HandlerContext<T>| Box::pin(handler(ctx)))
}

async fn handle_record<T: 'static + Clone>(
    req: HttpRequest,
    stream: Payload,
    state: Data<JambonzState<T>>,
) -> Result<HttpResponse, Error> {
    ws_response(&req, stream, state, "audio.jambonz.org")
}

async fn handle_ws<T: 'static + Clone>(
    req: HttpRequest,
    stream: Payload,
    state: Data<JambonzState<T>>,
) -> Result<HttpResponse, Error> {
    ws_response(&req, stream, state, "ws.jambonz.org")
}

fn ws_response<T: 'static + Clone>(
    req: &HttpRequest,
    stream: Payload,
    state: Data<JambonzState<T>>,
    protocol: &str,
) -> Result<HttpResponse, Error> {
    match actix_ws::handle(req, stream) {
        Ok(res) => {
            let (mut res, session, msg_stream) = res;
            rt::spawn(handler::handler(session, msg_stream, state));
            let ws_header = HeaderName::from_bytes("Sec-WebSocket-Protocol".to_string().as_bytes())
                .expect("should be valid WebSocket-Protocol");
            res.headers_mut().insert(
                ws_header.clone(),
                HeaderValue::from_bytes(protocol.as_bytes())?,
            );
            Ok(res)
        }
        Err(e) => {
            println!("{:?}", e);
            Ok(HttpResponse::InternalServerError().finish())
        }
    }
}

fn start_jambonz_server<T>(server: JambonzWebServer<T>) -> Server
where
    T: Clone + Send + 'static,
{
    let state = JambonzState {
        state: server.app_state.clone(),
        handler: server.handler.clone(),
    };

    HttpServer::new(move || {
        let d1 = Data::new(state.clone());

        App::new()
            .app_data(d1)
            .route(
                server.ws_path.clone().as_str(),
                web::get().to(handle_ws::<T>),
            )
            .route(
                server.record_path.clone().as_str(),
                web::get().to(handle_record::<T>),
            )
    })
    .bind((server.bind_ip, server.bind_port))
    .expect("Can not bind to server/port")
    .run()
}

#[derive(Clone)]
pub struct JambonzState<T> {
    pub state: T,
    pub handler: HandlerFn<T>,
}

pub enum JambonzRequest {
    TextMessage(WebsocketRequest),
    Binary(Vec<u8>),
    Close,
}

pub struct JambonzWebServer<T> {
    pub bind_ip: String,
    pub bind_port: u16,
    pub app_state: T,
    pub ws_path: String,
    pub record_path: String,
    pub handler: HandlerFn<T>,
}

impl<T: Clone + Send + 'static> JambonzWebServer<T> {
    pub fn new(app_state: T, handler: HandlerFn<T>) -> Self
    where
        T: Clone + Send + 'static,
    {
        JambonzWebServer {
            app_state,
            handler,
            bind_ip: "0.0.0.0".to_string(),
            bind_port: 8080,
            ws_path: "/ws".to_string(),
            record_path: "/record".to_string(),
        }
    }

    pub fn with_bind_ip(mut self, ip: impl Into<String>) -> Self {
        self.bind_ip = ip.into();
        self
    }

    pub fn with_bind_port(mut self, port: u16) -> Self {
        self.bind_port = port;
        self
    }

    pub fn with_ws_path(mut self, path: impl Into<String>) -> Self {
        self.ws_path = path.into();
        self
    }

    pub fn with_record_path(mut self, path: impl Into<String>) -> Self {
        self.record_path = path.into();
        self
    }

    pub async fn start(self) -> Server {
        start_jambonz_server(self)
    }
}

pub struct HandlerContext<T> {
    pub uuid: Uuid,
    pub session: Session,
    pub request: JambonzRequest,
    pub state: T,
}
