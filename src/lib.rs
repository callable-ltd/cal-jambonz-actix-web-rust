mod handler;

use actix_web::dev::Server;
use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::web::{resource, Data, Payload};
use actix_web::{middleware, rt, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_ws::Session;
use cal_jambonz::ws::WebsocketRequest;
use uuid::Uuid;

fn jambonz_handler<T, F: Fn(Uuid, Session, JambonzRequest, T)>(
    uuid: Uuid,
    session: Session,
    request: JambonzRequest,
    app_state: T,
    f: F,
) {
    f(uuid, session, request, app_state);
}

async fn handle_ws<T: 'static + Clone>(
    req: HttpRequest,
    stream: Payload,
    state: Data<JambonzState<T>>,
) -> Result<HttpResponse, Error> {
    ws_response(&req, stream, state, "ws.jambonz.org")
}

async fn handle_record<T: 'static + Clone>(
    req: HttpRequest,
    stream: Payload,
    state: Data<JambonzState<T>>,
) -> Result<HttpResponse, Error> {
    ws_response(&req, stream, state, "audio.jambonz.org")
}

fn ws_response<T: 'static + Clone>(
    req: &HttpRequest,
    stream: Payload,
    state: Data<JambonzState<T>>,
    protocol: &str,
) -> Result<HttpResponse, Error> {
    let result = actix_ws::handle(&req, stream)?;
    let (mut res, session, msg_stream) = result;
    rt::spawn(handler::echo_heartbeat_ws(
        session,
        msg_stream,
        state.clone(),
    ));
    let ws_header =
        HeaderName::from_bytes("Sec-WebSocket-Protocol".to_string().as_bytes())
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
    pub   bind_ip: String,
    pub  bind_port: u16,
    pub  app_state: T,
    pub  ws_path: String,
    pub  record_path: String,
    pub  handler: fn(Uuid, Session, JambonzRequest, T),
}

#[derive(Clone)]
pub struct JambonzState<T> {
    pub app_state: T,
    pub handler: fn(Uuid, Session, JambonzRequest, T),
}

impl<T: Send + Sync + 'static + Clone> JambonzWebServer<T> {
    pub async fn start(self) -> Server {
        HttpServer::new(move || {
            let state = Data::new(JambonzState {
                app_state: self.app_state.clone(),
                handler: self.handler.clone(),
            });
            App::new()
                .wrap(middleware::Logger::default())
                .app_data(state)
                .service(resource(self.ws_path.clone()).route(web::get().to(handle_ws::<T>)))
                .service(
                    resource(self.record_path.clone()).route(web::get().to(handle_record::<T>)),
                )
        })
        .bind((self.bind_ip, self.bind_port))
        .expect("Can not bind to server/port")
        .run()
    }
}
