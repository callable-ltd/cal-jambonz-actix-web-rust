mod handler;

use actix_web::dev::Server;
use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::web::{Data, Payload};
use actix_web::{rt, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_ws::Session;
use cal_jambonz::ws::WebsocketRequest;
use std::pin::Pin;
use std::sync::Arc;
use uuid::Uuid;

type HandlerFn<T> =
    Arc<dyn Fn(HandlerContext<T>) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

type WsHandler<T> = Arc<
    dyn Fn(
            HttpRequest,
            Payload,
            Data<T>,
        ) -> Pin<Box<dyn Future<Output = Result<HttpResponse, Error>> + Send>>
        + Send
        + Sync,
>;

pub fn register_handler<T, F, Fut>(handler: F) -> HandlerFn<T>
where
    T: Clone + Send + Sync + 'static,
    F: Fn(HandlerContext<T>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    Arc::new(move |ctx: HandlerContext<T>| Box::pin(handler(ctx)))
}

// async fn handle_record<T: 'static + Clone>(
//     req: HttpRequest,
//     stream: Payload,
//     state: Data<JambonzState<T>>,
// ) -> Result<HttpResponse, Error> {
//     ws_response(&req, stream, state, "audio.jambonz.org")
// }

async fn handle_ws<T: Clone + Send + Sync + 'static>(
    req: HttpRequest,
    stream: Payload,
    state: Data<T>,
    route: JambonzRoute<T>,
) -> Result<HttpResponse, Error> {
    let protocol = match route.ws_type {
        JambonzRouteType::Hook => "ws.jambonz.org",
        JambonzRouteType::Recording => "audio.jambonz.org"
    };
    ws_response(&req, stream, state, protocol, route.handler.into()).await
}

#[derive(Clone)]
pub struct JambonzRoute<T> {
    pub path: String,
    pub ws_type: JambonzRouteType,
    pub handler: Arc<dyn Fn(HandlerContext<T>) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>,
}

#[derive(Clone)]
pub enum JambonzRouteType {
    Hook,
    Recording,
}

async fn ws_response<T: Clone + 'static>(
    req: &HttpRequest,
    stream: Payload,
    state: Data<T>,
    protocol: &str,
    handler: Arc<HandlerFn<T>>,
) -> Result<HttpResponse, Error> {
    match actix_ws::handle(req, stream) {
        Ok((mut res, session, msg_stream)) => {
            res.headers_mut().insert(
                HeaderName::from_static("sec-websocket-protocol"),
                HeaderValue::from_str(protocol).expect("valid header value"),
            );
            rt::spawn(handler::handler(session, msg_stream, state, handler));
            Ok(res)
        }
        Err(e) => {
            println!("WebSocket error: {:?}", e);
            Ok(HttpResponse::InternalServerError().finish())
        }
    }
}

pub fn start_server<T>(server: JambonzWebServer<T>) -> Server
where
    T: Clone + Send + Sync + 'static,
{
    // Wrap routes in an Arc to share it safely across threads
    let routes_arc = Arc::new(server.routes.clone());

    HttpServer::new(move || {
        let mut app = App::new().app_data(Data::new(server.app_state.clone()));

        // Clone the Arc for each worker
        let routes_arc = Arc::clone(&routes_arc);

        // For each route, create a handler
        for route in routes_arc.iter() { // Use iter() to iterate over the Arc's content
            let path = route.path.clone();

            // Clone the entire route for use in the closure
            let route_clone = route.clone();

            // Create handler function with captured route clone
            let handler_fn = move |req, stream, state| {
                let route_clone = route_clone.clone(); // Clone again for the inner closure
                async move {
                    handle_ws(req, stream, state, route_clone).await
                }
            };

            // Register route
            app = app.route(&path, web::get().to(handler_fn));
        }

        app
    })
        .bind((server.bind_ip.clone(), server.bind_port))
        .expect("Can not bind to port")
        .run()
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
    pub routes: Vec<JambonzRoute<T>>,
}

impl<T: Clone + Send + 'static + Sync> JambonzWebServer<T> {
    pub fn new(app_state: T) -> Self
    where
        T: Clone + Send + 'static,
    {
        JambonzWebServer {
            app_state,
            bind_ip: "0.0.0.0".to_string(),
            bind_port: 8080,
            routes: vec![],
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
    pub fn add_route(mut self, route: JambonzRoute<T>) -> Self {
        self.routes.push(route);
        self
    }

    pub fn start(self) -> Server {
        start_server(self)
    }
}

pub struct HandlerContext<T> {
    pub uuid: Uuid,
    pub session: Session,
    pub request: JambonzRequest,
    pub state: Data<T>,
}

#[derive(Clone)]
pub struct TestData {
    pub message:String
}