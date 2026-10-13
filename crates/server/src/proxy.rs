//! Requests that arrive through a reverse proxy. The proxy tells us who the client is in `X-Forwarded-*` headers, which anyone can send:
//! they are believed only when the connection comes from an address in `TRUSTED_PROXIES`, and removed otherwise.

use std::{
  net::{IpAddr, SocketAddr},
  sync::Arc,
};

use axum::{
  extract::{ConnectInfo, Request, State},
  http::HeaderMap,
  middleware::Next,
  response::Response,
};
use ipnet::IpNet;
use tracing::Instrument;

/// The address ranges of the reverse proxies in front of this server.
#[derive(Debug, Clone, Default)]
pub(crate) struct TrustedProxies(Arc<Vec<IpNet>>);

/// The address of the client that made a request, as far as it can be known: the connection's peer, or what a trusted proxy reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClientIp(pub(crate) IpAddr);

/// What a proxy adds to a request.
const FORWARDED_HEADERS: [&str; 5] = ["x-forwarded-for", "x-forwarded-proto", "x-forwarded-host", "forwarded", "x-real-ip"];

impl TrustedProxies {
  /// A comma or space separated list of addresses and CIDR ranges (`10.0.0.0/8, 172.28.0.2`).
  pub(crate) fn parse(list: &str) -> Result<Self, String> {
    let ranges = list
      .split([',', ' '])
      .filter(|item| !item.is_empty())
      .map(|item| {
        item
          .parse::<IpNet>()
          .or_else(|_| item.parse::<IpAddr>().map(IpNet::from))
          .map_err(|_| format!("{item} is not an address or a range like 10.0.0.0/8"))
      })
      .collect::<Result<_, _>>()?;

    Ok(Self(Arc::new(ranges)))
  }

  fn contains(&self, ip: IpAddr) -> bool {
    self.0.iter().any(|range| range.contains(&ip))
  }

  /// The client behind a connection from `peer`. A trusted peer is a proxy: the client is the last address in `X-Forwarded-For` that
  /// is not itself a trusted proxy (the entries to its left are what the client claimed, which is not evidence).
  fn client(&self, peer: IpAddr, headers: &HeaderMap) -> IpAddr {
    if !self.contains(peer) {
      return peer;
    }

    let chain = headers.get_all("x-forwarded-for").iter().filter_map(|v| v.to_str().ok()).flat_map(|v| v.split(','));

    chain.filter_map(|entry| entry.trim().parse::<IpAddr>().ok()).rev().find(|ip| !self.contains(*ip)).unwrap_or(peer)
  }
}

/// Middleware: works out the client, drops forwarded headers that cannot be believed, and makes the client part of every log line of the request.
pub(crate) async fn forwarded(State(trusted): State<TrustedProxies>, ConnectInfo(peer): ConnectInfo<SocketAddr>, mut request: Request, next: Next) -> Response {
  let client = trusted.client(peer.ip(), request.headers());

  if !trusted.contains(peer.ip()) {
    for name in FORWARDED_HEADERS {
      request.headers_mut().remove(name);
    }
  }
  request.extensions_mut().insert(ClientIp(client));

  next.run(request).instrument(tracing::info_span!("request", client_ip = %client)).await
}

#[cfg(test)]
mod tests {
  use axum::http::HeaderValue;

  use super::*;

  fn headers(forwarded_for: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("x-forwarded-for", HeaderValue::from_str(forwarded_for).unwrap());
    headers
  }

  #[test]
  fn forwarded_addresses_are_believed_only_from_a_trusted_proxy() {
    let trusted = TrustedProxies::parse("172.28.0.0/24, 10.1.1.1").unwrap();

    // (peer, X-Forwarded-For, client)
    for (peer, chain, client) in [
      // Directly connected: whatever the header says is a claim.
      ("203.0.113.9", "198.51.100.1", "203.0.113.9"),
      // Behind the proxy: its report is the client.
      ("172.28.0.2", "198.51.100.1", "198.51.100.1"),
      // The client sent its own header first; the proxy appended the real address.
      ("172.28.0.2", "1.2.3.4, 198.51.100.1", "198.51.100.1"),
      // Several proxies in a row.
      ("172.28.0.2", "198.51.100.1, 10.1.1.1", "198.51.100.1"),
      // Nothing usable: the proxy itself.
      ("172.28.0.2", "garbage", "172.28.0.2"),
      ("10.1.1.1", "", "10.1.1.1"),
    ] {
      let found = trusted.client(peer.parse().unwrap(), &headers(chain));

      assert_eq!(found, client.parse::<IpAddr>().unwrap(), "{peer} {chain}");
    }
  }

  #[test]
  fn nobody_is_trusted_by_default() {
    let nobody = TrustedProxies::default();

    assert_eq!(nobody.client("127.0.0.1".parse().unwrap(), &headers("198.51.100.1")), "127.0.0.1".parse::<IpAddr>().unwrap());
    assert!(TrustedProxies::parse("10.0.0.0/33").is_err());
    assert!(TrustedProxies::parse("").unwrap().0.is_empty());
  }

  #[tokio::test]
  async fn untrusted_peers_lose_their_forwarded_headers() {
    use axum::{Router, body::Body, extract::connect_info::MockConnectInfo, http::Request, middleware, routing::get};
    use tower::ServiceExt;

    let app = |peer: &str, trusted: &str| {
      Router::new()
        .route(
          "/",
          get(|headers: HeaderMap| async move { headers.get("x-forwarded-for").map_or("none".to_string(), |v| v.to_str().unwrap().to_string()) }),
        )
        .layer(middleware::from_fn_with_state(TrustedProxies::parse(trusted).unwrap(), forwarded))
        .layer(MockConnectInfo(SocketAddr::new(peer.parse().unwrap(), 4000)))
    };
    let ask = |app: Router| async move {
      let response = app.oneshot(Request::get("/").header("x-forwarded-for", "1.2.3.4").body(Body::empty()).unwrap()).await.unwrap();
      String::from_utf8(axum::body::to_bytes(response.into_body(), 1024).await.unwrap().to_vec()).unwrap()
    };

    assert_eq!(ask(app("203.0.113.9", "172.28.0.0/24")).await, "none");
    assert_eq!(ask(app("172.28.0.2", "172.28.0.0/24")).await, "1.2.3.4");
  }
}
