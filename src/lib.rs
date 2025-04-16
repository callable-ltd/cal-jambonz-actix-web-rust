mod handler;

use actix_web::dev::Server;
use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::web::{Data, Payload};
use actix_web::{rt, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_ws::Session;
use cal_jambonz::ws::WebsocketRequest;
use uuid::Uuid;

async fn handle_ws<T: 'static + Clone, U: Fn(Uuid, Session, JambonzRequest, Data<T>) + 'static>(
    req: HttpRequest,
    stream: Payload,
    state: Data<T>,
    handler: Data<U>,
) -> Result<HttpResponse, Error> {
    ws_response(&req, stream, state, handler, "ws.jambonz.org")
}

async fn handle_record<
    T: 'static + Clone,
    U: Fn(Uuid, Session, JambonzRequest, Data<T>) + 'static,
>(
    req: HttpRequest,
    stream: Payload,
    state: Data<T>,
    handler: Data<U>,
) -> Result<HttpResponse, Error> {
    ws_response(&req, stream, state, handler, "audio.jambonz.org")
}

fn ws_response<T: 'static + Clone, U: Fn(Uuid, Session, JambonzRequest, Data<T>) + 'static>(
    req: &HttpRequest,
    stream: Payload,
    state: Data<T>,
    handler: Data<U>,
    protocol: &str,
) -> Result<HttpResponse, Error> {
    let result = actix_ws::handle(&req, stream)?;
    let (mut res, session, msg_stream) = result;

    rt::spawn(handler::echo_heartbeat_ws(
        session,
        msg_stream,
        state.clone(),
        handler.clone(),
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

pub struct JambonzWebServer<T> {
    pub bind_ip: String,
    pub bind_port: u16,
    pub app_state: T,
    pub ws_path: String,
    pub record_path: String,
}

pub fn start_jambonz_server<
    T: Clone + Send + 'static,
    U: Future + Fn(Uuid, Session, JambonzRequest, Data<T>) + 'static,
>(
    server: JambonzWebServer<T>,
    handler: fn(Uuid, Session, JambonzRequest, T) -> U,
) -> Server {
    HttpServer::new(move || {
        let d1 = Data::new(handler);
        let d2 = Data::new(server.app_state.clone());

        App::new()
            .app_data(d1)
            .app_data(d2)
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
