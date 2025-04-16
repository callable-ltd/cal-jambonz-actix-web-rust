mod handler;

use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::web::{Data, Payload, resource};
use actix_web::{App, Error, HttpRequest, HttpResponse, HttpServer, rt, web};
use actix_web::{middleware::Logger};
use actix_ws::{AggregatedMessage, Session};
use cal_jambonz::ws::WebsocketRequest;
use futures_util::StreamExt;
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
    pub handler: fn(Uuid, Session, JambonzRequest, T),
}

#[derive(Clone)]
pub struct JambonzState<T> {
    pub app_state: T,
    pub handler: fn(Uuid, Session, JambonzRequest, T),
}


async fn echo(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    let (res, mut session, stream) = actix_ws::handle(&req, stream)?;

    let mut stream = stream
        .aggregate_continuations()
        // aggregate continuation frames up to 1MiB
        .max_continuation_size(2_usize.pow(20));

    // start task but don't wait for it
    rt::spawn(async move {
        // receive messages from websocket
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(AggregatedMessage::Text(text)) => {
                    // echo text message
                    session.text(text).await.unwrap();
                }

                Ok(AggregatedMessage::Binary(bin)) => {
                    // echo binary message
                    session.binary(bin).await.unwrap();
                }

                Ok(AggregatedMessage::Ping(msg)) => {
                    // respond to PING frame with PONG frame
                    session.pong(&msg).await.unwrap();
                }

                _ => {}
            }
        }
    });

    // respond immediately with response connected to WS session
    Ok(res)
}

impl<T: Send + Sync + 'static + Clone> JambonzWebServer<T> {
    pub async fn start(self) -> std::io::Result<()> {
        HttpServer::new(move || {
            let state = Data::new(JambonzState {
                app_state: self.app_state.clone(),
                handler: self.handler.clone(),
            });
            App::new()
                .app_data(state)
                .route(self.ws_path.clone().as_str(), web::get().to(echo))
                // .service(
                //     resource(self.record_path.clone()).route(web::get().to(handle_record::<T>)),
                // )
        })
        .bind((self.bind_ip, self.bind_port))
        .expect("Can not bind to server/port")
        .run()
        .await
    }
}
