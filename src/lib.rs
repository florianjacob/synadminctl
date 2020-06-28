use std::convert::TryInto;
use thiserror::Error;
use std::sync::Arc;

pub mod endpoints;
pub use endpoints::*;

use async_trait::async_trait;

// difference from tower service:
// A) async_trait, so that there isn't a manual third `Future` associated type
// B) call is &self, i.e. not mut, so that we do not need to clone for exclusive ownership
#[async_trait]
pub trait Service<Request> {
    type Response;
    type Error;

    async fn call(&self, _: Request) -> Result<Self::Response, Self::Error>;
}


// this would be RumaClientError if this was an actual Matrix library,
// and should not contain Synadminctl-specific errors
#[derive(Error, Debug)]
pub enum MatrixLibError {
    #[error("received malformed json")]
    Deserialization(#[from] serde_json::Error),
    #[error("http response had unexpected error")]
    Http(http::Response<Vec<u8>>),
    #[error("error when calling http")]
    HttpService(#[from] anyhow::Error),
}

#[derive(Clone)]
pub struct AnonymousMatrixService<T> {
    inner: Arc<InnerAnonymousMatrixService<T>>,
}

struct InnerAnonymousMatrixService<T> {
    http_service: T,
    server_uri: http::Uri,
}

impl<S> AnonymousMatrixService<S>
where
    // TODO: this would benefit from trait aliases: https://github.com/rust-lang/rust/issues/63063
    S: Service<http::Request<Vec<u8>>, Response=http::Response<Vec<u8>>, Error=anyhow::Error> + Send + Sync,
{
    pub fn new(http_service: S, server_uri: http::Uri) -> AnonymousMatrixService<S> {
        Self {
            inner: Arc::new(InnerAnonymousMatrixService {
                http_service,
                server_uri,
            }),
        }
    }
}

#[async_trait]
impl<Request, S> Service<Request> for AnonymousMatrixService<S>
where
    Request: Endpoint + Send + 'static,
    S: Service<http::Request<Vec<u8>>, Response=http::Response<Vec<u8>>, Error=anyhow::Error> + Send + Sync,
{
    type Response = Request::Response;
    type Error = MatrixLibError;

    // TODO: while this obviously implements Service for Request::Endpoint, implementing that trait utterly breaks
    // and I don't understand why, probably due to #[async_trait] implementation awkwardnesses
    async fn call(&self, request: Request) -> Result<Self::Response, Self::Error> {
        let mut http_request: http::Request<Vec<u8>> = request.into();
        assert!(!Request::REQUIRES_AUTHENTICATION);

        let mut server_parts = self.inner.server_uri.clone().into_parts();
        server_parts.path_and_query = http_request.uri().path_and_query().cloned();
        *http_request.uri_mut() = http::Uri::from_parts(server_parts).unwrap();

        let http_response = self.inner.http_service.call(http_request).await?;

        let matrix_response = http_response.try_into();
        matrix_response
    }

}


#[derive(Clone, Debug, serde::Deserialize, Eq, Hash, PartialEq, serde::Serialize)]
pub struct Session {
    pub server_uri: String,
    /// The user the access token was issued for.
    pub user_id: String,
    /// The access token used for this session.
    pub access_token: String,
    /// The ID of the client device
    pub device_id: String,
}

#[derive(Clone)]
pub struct MatrixService<S> {
    inner: Arc<InnerMatrixService<S>>,
}
struct InnerMatrixService<S> {
    http_service: S,
    server_uri: http::Uri,
    authorization_header_value: http::HeaderValue,
}

impl<S> MatrixService<S>
where
    S: Service<http::Request<Vec<u8>>, Response=http::Response<Vec<u8>>, Error=anyhow::Error>,
{
    pub fn new(http_service: S, server_uri: http::Uri, access_token: String) -> MatrixService<S> {
        let authorization_string = format!("Bearer {}", access_token);
        let authorization_header_value = http::HeaderValue::from_str(&authorization_string).unwrap();
        Self {
            inner: Arc::new(InnerMatrixService {
                http_service,
                server_uri,
                authorization_header_value,
            }),
        }
    }
}

#[async_trait]
impl<Request, S> Service<Request> for MatrixService<S>
where
    Request: Endpoint + Send + 'static,
    S: Service<http::Request<Vec<u8>>, Response=http::Response<Vec<u8>>, Error=anyhow::Error> + Send + Sync,
{
    type Response = Request::Response;
    type Error = MatrixLibError;

    async fn call(&self, request: Request) -> Result<Self::Response, Self::Error> {
        let mut http_request: http::Request<Vec<u8>> = request.into();
        if Request::REQUIRES_AUTHENTICATION {
            http_request.headers_mut().insert("Authorization", self.inner.authorization_header_value.clone());
        }

        let mut server_parts = self.inner.server_uri.clone().into_parts();
        server_parts.path_and_query = http_request.uri().path_and_query().cloned();
        *http_request.uri_mut() = http::Uri::from_parts(server_parts).unwrap();

        let http_response = self.inner.http_service.call(http_request).await?;

        let matrix_response = http_response.try_into();
        matrix_response
    }
}



#[derive(Debug, Eq, PartialEq)]
pub enum AutoDiscoveryError {
    /// Retrieve the specific piece of information from the user in a way which fits within the
    /// existing client user experience, if the client is inclined to do so. Failure can take place
    /// instead if no good user experience for this is possible at this point.
    Prompt,
    /// Stop the current auto-discovery mechanism. If no more auto-discovery mechanisms are
    /// available, then the client may use other methods of determining the required parameters,
    /// such as prompting the user, or using default values.
    // Ignore should not actually be returned to the API user,
    // so it should not actually appear here…
    // at least as long as we offer a full autodiscovery with all methods,
    // instead of separate functions for separate methods
    Ignore,
    /// Inform the user that auto-discovery failed due to invalid/empty data and PROMPT for the parameter.
    FailPrompt(String),
    /// Inform the user that auto-discovery did not return any usable URLs. Do not continue further
    /// with the current login process. At this point, valid data was obtained, but no server is
    /// available to serve the client. No further guess should be attempted and the user should make
    /// a conscientious decision what to do next.
    FailError(String),
}

pub async fn server_discovery<S>(http_service: S, user_id: String) -> Result<DiscoveryInfo, AutoDiscoveryError>
    where S: Service<http::Request<Vec<u8>>, Response=http::Response<Vec<u8>>, Error=anyhow::Error> + Clone + Send + Sync
{
    // https://matrix.org/docs/spec/client_server/latest#well-known-uri
    // 1. Extract the server name from the user's Matrix ID by splitting the Matrix ID at the first colon.
    // 2. Extract the hostname from the server name.
    // Grammar:
    // server_name = hostname [ ":" port ]
    // user_id = "@" user_id_localpart ":" server_name
    let parts: Vec<&str> = user_id.split(':').collect();
    if parts.len() != 2 && parts.len() != 3 {
        // user_id is not a full user_id but just a username or something like that,
        // so a hostname cannot be extracted
        dbg!("invalid userid format: {}, {}", parts.len(), parts);
        return Err(AutoDiscoveryError::Prompt);
    }
    let hostname = parts[1];
    // 3. Make a GET request to https://hostname/.well-known/matrix/client.
    let domain = "https://".to_string() + hostname;
    let service = AnonymousMatrixService::new(http_service.clone(), domain.parse().unwrap());

    // 3. Make a GET request to https://hostname/.well-known/matrix/client.
    // 3c. Parse the response body as a JSON object
    let discovery_response = service.call(DiscoveryRequest).await;

    let discovery_info = match discovery_response {
        // 3a. If the returned status code is 404, then IGNORE.
        Ok(DiscoveryResponse::None) => Err(AutoDiscoveryError::Ignore),
        // 3b. If the returned status code is not 200, or the response body is empty, then FAIL_PROMPT.
        Err(MatrixLibError::Http(error_response)) => Err(AutoDiscoveryError::FailPrompt(format!("{}", error_response.status()))),
        // 3ci. If the content cannot be parsed, then FAIL_PROMPT.
        // 3di. If this value is not provided, then FAIL_PROMPT.
        Err(MatrixLibError::Deserialization(source_error)) => Err(AutoDiscoveryError::FailPrompt(format!("{}", source_error))),
        Err(MatrixLibError::HttpService(source_error)) => Err(AutoDiscoveryError::FailPrompt(format!("{}", source_error))),
        Ok(DiscoveryResponse::Some(discovery_info)) => Ok(discovery_info),
    };


    // this is our only autodiscovery mechanism,
    // therefore map Ignore to Prompt and possibly return
    let discovery_info = discovery_info.map_err(
        |error| if error == AutoDiscoveryError::Ignore { dbg!("no valid record"); AutoDiscoveryError::Prompt } else { error })?;


    // 3d. Extract the base_url value from the m.homeserver property.
    //     This value is to be used as the base URL of the homeserver.
    // 3e. Validate the homeserver base URL:
    // 3ei. Parse it as a URL. If it is not a URL, then FAIL_ERROR.
    let base_url = discovery_info.homeserver.base_url.parse()
        .map_err(|error| AutoDiscoveryError::FailError(format!("{}", error)))?;
    let service = AnonymousMatrixService::new(http_service.clone(), base_url);
    // 3eii. Clients SHOULD validate that the URL points to a valid homeserver before accepting it
    //     by connecting to the /_matrix/client/versions endpoint,
    //     ensuring that it does not return an error,
    //     and parsing and validating that the data conforms with the expected response format.
    //     If any step in the validation fails, then FAIL_ERROR.
    //     Validation is done as a simple check against configuration errors,
    //     in order to ensure that the discovered address points to a valid homeserver.
    let _ = service.call(ClientVersionRequest).await
        .map_err(|error| AutoDiscoveryError::FailError(format!("{}", error)))?;


    // If the m.identity_server property is present, extract the base_url value for use as the
    // base URL of the identity server. Validation for this URL is done as in the step above,
    // but using /_matrix/identity/api/v1 as the endpoint to connect to. If the
    // m.identity_server property is present, but does not have a base_url value, then
    // FAIL_ERROR.
    if let Some(identity_server_info) = &discovery_info.identity_server {
        let base_url = identity_server_info.base_url.parse()
            .map_err(|error| AutoDiscoveryError::FailError(format!("{}", error)))?;
        let service = AnonymousMatrixService::new(http_service.clone(), base_url);
        let identity_status_response = service.call(IdentityStatusRequest).await;

        identity_status_response
            .and(Ok(discovery_info))
            .map_err(|error| AutoDiscoveryError::FailError(format!("{}", error)))
    }
    else {
        Ok(discovery_info)
    }
}


// TODO: try out a Paging API
