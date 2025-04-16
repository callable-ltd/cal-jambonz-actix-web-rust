mod handler;

use actix_web::dev::Server;
use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::web::{Data, Payload};
use actix_web::{App, Error, HttpRequest, HttpResponse, HttpServer, rt, web};
use actix_ws::Session;
use cal_jambonz::ws::WebsocketRequest;
use std::pin::Pin;
use uuid::Uuid;

async fn handle_ws<
    T: 'static + Clone,
    U: Fn(Uuid, Session, JambonzRequest, T) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
        + Clone + 'static,
>(
    req: HttpRequest,
    stream: Payload,
    state: Data<JambonzState<T, U>>,
) -> Result<HttpResponse, Error> {
    ws_response(&req, stream, state, "ws.jambonz.org")
}

async fn handle_record<
    T: 'static + Clone,
    U: Fn(Uuid, Session, JambonzRequest, T) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
        + Clone + 'static,
>(
    req: HttpRequest,
    stream: Payload,
    state: Data<JambonzState<T, U>>,
) -> Result<HttpResponse, Error> {
    ws_response(&req, stream, state, "audio.jambonz.org")
}

fn ws_response<
    T: 'static + Clone,
    U: Fn(Uuid, Session, JambonzRequest, T) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
        + Clone + 'static,
>(
    req: &HttpRequest,
    stream: Payload,
    state: Data<JambonzState<T, U>>,
    protocol: &str,
) -> Result<HttpResponse, Error> {
    let result = actix_ws::handle(&req, stream)?;
    let (mut res, session, msg_stream) = result;
    rt::spawn(handler::echo_heartbeat_ws(
        session,
        msg_stream,
        state.clone(),
    ));
    let ws_header = HeaderName::from_bytes("Sec-WebSocket-Protocol".to_string().as_bytes())
        .expect("should be valid WebSocket-Protocol");

    res.headers_mut().insert(
        ws_header.clone(),
        HeaderValue::from_bytes(protocol.as_bytes()).expect("Could not parse protocol header"),
    );
    Ok(res)
}

pub enum JambonzRequest {
    TextMessage(WebsocketRequest),
    Binary(Vec<u8>),
    Close,
}

pub struct JambonzWebServer<T, U>
where
    U: Fn(Uuid, Session, JambonzRequest, T) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
{
    pub bind_ip: String,
    pub bind_port: u16,
    pub app_state: T,
    pub ws_path: String,
    pub record_path: String,
    pub handler: U,
}

#[derive(Clone)]
pub struct JambonzState<T, U>
where
    U: Fn(Uuid, Session, JambonzRequest, T) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
{
    pub app_state: T,
    pub handler: U,
}

pub fn start_jambonz_server<
    T: Clone + Send + 'static,
    U: Fn(Uuid, Session, JambonzRequest, T) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
        + Clone
        + Send
        + 'static,
>(
    server: JambonzWebServer<T, U>,
) -> Server {
    HttpServer::new(move || {
        let state = Data::new(JambonzState {
            app_state: server.app_state.clone(),
            handler: server.handler.clone(),
        });
        App::new()
            .app_data(state)
            .route(
                server.ws_path.clone().as_str(),
                web::get().to(handle_ws::<T, U>),
            )
            .route(
                server.record_path.clone().as_str(),
                web::get().to(handle_record::<T, U>),
            )
    })
    .bind((server.bind_ip, server.bind_port))
    .expect("Can not bind to server/port")
    .run()
}
