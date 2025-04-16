use crate::{JambonzRequest, JambonzState};
use actix_web::web::Data;
use actix_ws::{Message, Session};
use futures_util::{
    future::{self, Either},
    StreamExt as _,
};
use std::pin::Pin;
use std::time::{Duration, Instant};
use tokio::{pin, time::interval};
use uuid::Uuid;

/// How often heartbeat pings are sent.
///
/// Should be half (or less) of the acceptable client timeout.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// How long before lack of client response causes a timeout.
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn echo_heartbeat_ws<
    T: 'static + Clone,
    U: Fn(Uuid, Session, JambonzRequest, T) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
        + Clone,
>(
    mut session: Session,
    mut msg_stream: actix_ws::MessageStream,
    state: Data<JambonzState<T, U>>,
) {
    let uuid = Uuid::new_v4();
    println!("Handler:new_session: {}", uuid.to_string());

    let mut last_heartbeat = Instant::now();
    let mut interval = interval(HEARTBEAT_INTERVAL);

    let reason = loop {
        // create "next client timeout check" future
        let tick = interval.tick();
        // required for select()
        pin!(tick);

        // waits for either `msg_stream` to receive a message from the client or the heartbeat
        // interval timer to tick, yielding the value of whichever one is ready first
        match future::select(msg_stream.next(), tick).await {
            // received message from WebSocket client
            Either::Left((Some(Ok(msg)), _)) => {
                match msg {
                    Message::Text(text) => match serde_json::from_str(&text) {
                        Ok(json) => {
                            let req = JambonzRequest::TextMessage(json);
                            jambonz_handler(
                                uuid,
                                session.clone(),
                                req,
                                state.app_state.clone(),
                                state.handler.clone(),
                            );
                        }
                        Err(e) => {
                            println!("Error reading TextMessage: {}", e);
                        }
                    },

                    Message::Binary(bytes) => {
                        let req = JambonzRequest::Binary(bytes.into_iter().collect::<Vec<_>>());
                        jambonz_handler(
                            uuid,
                            session.clone(),
                            req,
                            state.app_state.clone(),
                            state.handler.clone(),
                        );
                    }

                    Message::Close(reason) => {
                        let req = JambonzRequest::Close;
                        jambonz_handler(
                            uuid,
                            session.clone(),
                            req,
                            state.app_state.clone(),
                            state.handler.clone(),
                        );
                        break reason;
                    }

                    Message::Ping(bytes) => {
                        last_heartbeat = Instant::now();
                        let _ = session.pong(&bytes).await;
                    }

                    Message::Pong(_) => {
                        last_heartbeat = Instant::now();
                    }

                    Message::Continuation(_) => {
                        println!("no support for continuation frames");
                    }

                    // no-op; ignore
                    Message::Nop => {}
                };
            }

            // client WebSocket stream error
            Either::Left((Some(Err(err)), _)) => {
                println!("{}", err);
                break None;
            }

            // client WebSocket stream ended
            Either::Left((None, _)) => break None,

            // heartbeat interval ticked
            Either::Right((_inst, _)) => {
                // if no heartbeat ping/pong received recently, close the connection
                if Instant::now().duration_since(last_heartbeat) > CLIENT_TIMEOUT {
                    println!(
                        "client has not sent heartbeat in over {CLIENT_TIMEOUT:?}; disconnecting"
                    );
                    break None;
                }

                // send heartbeat ping
                let _ = session.ping(b"").await;
            }
        };
    };

    // attempt to close connection gracefully
    let _ = session.close(reason).await;
}

fn jambonz_handler<
    T: 'static + Clone,
    F: Fn(Uuid, Session, JambonzRequest, T) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
>(
    uuid: Uuid,
    session: Session,
    request: JambonzRequest,
    app_state: T,
    f: F,
) {
    f(uuid, session, request, app_state);
}
