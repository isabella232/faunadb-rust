//! Tools for communicating with Fauna.

mod response;

#[cfg(feature = "sync_client")]
mod sync;

pub use response::*;

#[cfg(feature = "sync_client")]
pub use sync::*;

use crate::{
    error::{Error, FaunaErrors},
    expr::Expr,
};
use futures::{future, stream::Stream, Future};
use http::header::{AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE};
use hyper::{client::HttpConnector, Body, StatusCode, Uri};
use hyper_tls::HttpsConnector;
use serde_json;
use std::{borrow::Cow, time::Duration};
use tokio_timer::Timeout;

type Transport = hyper::Client<HttpsConnector<HttpConnector>>;

/// For building a new Fauna client.
pub struct ClientBuilder<'a> {
    uri: Cow<'a, str>,
    secret: Cow<'a, str>,
    timeout: Duration,
}

impl<'a> ClientBuilder<'a> {
    /// Change the uri if using dedicated Fauna servers. Default:
    /// `https://db.fauna.com`.
    pub fn uri(&mut self, uri: impl Into<Cow<'a, str>>) -> &mut Self {
        self.uri = uri.into();
        self
    }

    /// Request timeout. Default: `60 seconds`.
    pub fn timeout(&mut self, timeout: Duration) -> &mut Self {
        self.timeout = timeout;
        self
    }

    /// Creates the client.
    pub fn build(self) -> crate::Result<Client> {
        let mut builder = hyper::Client::builder();
        builder.keep_alive(true);

        let secret_b64 = base64::encode(&format!("{}:", self.secret));

        Ok(Client {
            transport: builder.build(HttpsConnector::new(1)?),
            uri: self.uri.parse()?,
            timeout: self.timeout,
            authorization: format!("Basic {}", secret_b64),
        })
    }

    #[cfg(feature = "sync_client")]
    pub fn build_sync(self) -> crate::Result<SyncClient> {
        Ok(SyncClient::new(self.build()?)?)
    }
}

/// The client for Fauna. Should be created using the
/// [ClientBuilder](struct.ClientBuilder.html).
///
/// Do not create new clients for every request to prevent
/// spamming Fauna servers with new connections.
pub struct Client {
    transport: Transport,
    uri: Uri,
    timeout: Duration,
    authorization: String,
}

impl Client {
    /// Create a new client builder. Secret can be generated in [Fauna Cloud
    /// Console](https://dashboard.fauna.com/keys-new/@db/).
    pub fn builder<'a>(secret: impl Into<Cow<'a, str>>) -> ClientBuilder<'a> {
        ClientBuilder {
            uri: Cow::from("https://db.fauna.com"),
            secret: secret.into(),
            timeout: Duration::new(60, 0),
        }
    }

    /// Send a query to Fauna servers and parsing the response.
    pub fn query<'a, Q>(&self, query: Q) -> FutureResponse<Response>
    where
        Q: Into<Expr<'a>>,
    {
        let query = query.into();
        let payload_json = serde_json::to_string(&query).unwrap();

        trace!("Querying with: {:?}", &payload_json);

        self.request(self.build_request(payload_json), |body| {
            serde_json::from_str(&body).unwrap()
        })
    }

    fn request<F, T>(&self, request: hyper::Request<Body>, f: F) -> FutureResponse<T>
    where
        T: Send + Sync + 'static,
        F: FnOnce(String) -> T + Send + Sync + 'static,
    {
        let send_request = self
            .transport
            .request(request)
            .map_err(|e| Error::ConnectionError(e.into()));

        let requesting = send_request.and_then(move |response| {
            trace!("Client::call got response status {}", response.status());

            let status = response.status();

            let get_body = response
                .into_body()
                .map_err(|e| Error::ConnectionError(e.into()))
                .concat2();

            get_body.and_then(move |body_chunk| {
                if let Ok(body) = String::from_utf8(body_chunk.to_vec()) {
                    trace!("Got response: {:?}", &body);

                    match status {
                        s if s.is_success() => future::ok(f(body)),
                        StatusCode::UNAUTHORIZED => future::err(Error::Unauthorized),
                        StatusCode::BAD_REQUEST => {
                            let errors: FaunaErrors = serde_json::from_str(&body).unwrap();
                            future::err(Error::BadRequest(errors))
                        }
                        StatusCode::NOT_FOUND => {
                            let errors: FaunaErrors = serde_json::from_str(&body).unwrap();
                            future::err(Error::NotFound(errors))
                        }
                        _ => future::err(Error::DatabaseError(body)),
                    }
                } else {
                    future::err(Error::EmptyResponse)
                }
            })
        });

        let with_timeout = Timeout::new(requesting, self.timeout).map_err(|e| {
            if e.is_timer() {
                Error::TimeoutError
            } else {
                match e.into_inner() {
                    Some(error) => error,
                    None => Error::Other,
                }
            }
        });

        FutureResponse(Box::new(with_timeout))
    }

    fn build_request(&self, payload: String) -> hyper::Request<Body> {
        let mut builder = hyper::Request::builder();

        builder.uri(&self.uri);
        builder.method("POST");

        builder.header(CONTENT_LENGTH, format!("{}", payload.len()).as_bytes());
        builder.header(CONTENT_TYPE, "application/json");
        builder.header(AUTHORIZATION, self.authorization.as_bytes());
        builder.header("X-FaunaDB-API-Version", "2.1");

        builder.body(Body::from(payload)).unwrap()
    }
}
