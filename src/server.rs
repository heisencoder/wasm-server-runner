use std::net::SocketAddr;

use axum::handler::get;
use axum::http::{HeaderValue, Response};
use axum::response::{Html, IntoResponse};
use axum::Router;
use tower::ServiceBuilder;

use crate::wasm_bindgen::WasmBindgenOutput;
use crate::Result;

pub async fn run_server(output: WasmBindgenOutput) -> Result<()> {
    let WasmBindgenOutput { js, wasm } = output;

    let middleware_stack = ServiceBuilder::new().into_inner();

    let app = Router::new()
        .route("/", get(|| async { Html(include_str!("../static/index.html")) }))
        .route("/wasm.js", get(|| async { WithContentType("application/javascript", js) }))
        .route("/wasm.wasm", get(|| async { WithContentType("application/wasm", wasm) }))
        .layer(middleware_stack);

    let port = pick_port::pick_free_port(1334, 10).unwrap_or(1334);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    tracing::info!("starting webserver at http://{}", addr);
    axum::Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}

struct WithContentType<T>(&'static str, T);
impl<T: IntoResponse> IntoResponse for WithContentType<T> {
    type Body = T::Body;
    type BodyError = T::BodyError;

    fn into_response(self) -> Response<Self::Body> {
        let mut response = self.1.into_response();
        response.headers_mut().insert("Content-Type", HeaderValue::from_static(self.0));
        response
    }
}

mod pick_port {
    use std::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6, TcpListener, ToSocketAddrs};

    fn test_bind_tcp<A: ToSocketAddrs>(addr: A) -> Option<u16> {
        Some(TcpListener::bind(addr).ok()?.local_addr().ok()?.port())
    }
    fn is_free_tcp(port: u16) -> bool {
        let ipv4 = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);
        let ipv6 = SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, port, 0, 0);

        test_bind_tcp(ipv6).is_some() && test_bind_tcp(ipv4).is_some()
    }

    fn ask_free_tcp_port() -> Option<u16> {
        let ipv4 = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0);
        let ipv6 = SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0);
        test_bind_tcp(ipv6).or_else(|| test_bind_tcp(ipv4))
    }

    pub fn pick_free_port(starting_at: u16, try_consecutive: u16) -> Option<u16> {
        (starting_at..=starting_at + try_consecutive)
            .find(|&port| is_free_tcp(port))
            .or_else(ask_free_tcp_port)
    }
}