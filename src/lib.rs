#![warn(rust_2018_idioms, missing_debug_implementations)]

use std::convert::TryInto;
use thiserror::Error;
use std::sync::Arc;
use std::ops::Deref;

pub mod endpoints;
pub mod http_services;
pub use endpoints::*;


use async_trait::async_trait;

// difference from tower service:
// A) async_trait, so that there isn't a manual third `Future` associated type
// B) call is &self, i.e. not mut, so that we do not need to clone for exclusive ownership
#[async_trait]
pub trait Service<Request> {
    type Response;
    type Error;

    async fn call(&self, _: Request) -> Result<Self::Response, Self::Error>
        where Request: 'async_trait;
}


// this would be RumaClientError if this was an actual Matrix library,
// and should not contain Synadminctl-specific errors
#[derive(Error, Debug)]
pub enum MatrixLibError<E: std::error::Error + 'static> {
    // #[error("received malformed json")]
    // Deserialization(#[from] serde_json::Error),
    // TODO: this serializes as actual array of ints, should be deserialized into a string somehow
    // #[error("http response had unexpected error")]
    // Http(http::Response<Vec<u8>>),
    // TODO: das hier ist auch z.B. 504 Gateway Timeout
    // TODO: also a [400 / M_UNRECOGNIZED]
    #[error("error when parsing http response")]
    FromHttpResponseError(#[from] ruma::api::error::FromHttpResponseError<E>),
    #[error("error when converting to http request")]
    IntoHttpError(#[from] ruma::api::error::IntoHttpError),
    #[error("error when calling http")]
    HttpService(#[from] anyhow::Error),
}

#[derive(Clone, Debug)]
pub struct AnonymousMatrixService<S> {
    inner: Arc<InnerAnonymousMatrixService<S>>,
}

#[derive(Debug)]
struct InnerAnonymousMatrixService<S> {
    http_service: S,
    base_url: String,
}

impl<S> AnonymousMatrixService<S>
where
    // TODO: this would benefit from trait aliases: https://github.com/rust-lang/rust/issues/63063
    S: Service<http::Request<Vec<u8>>, Response=http::Response<Vec<u8>>, Error=anyhow::Error> + Send + Sync,
{
    pub fn new(http_service: S, base_url: String) -> AnonymousMatrixService<S> {
        Self {
            inner: Arc::new(InnerAnonymousMatrixService {
                http_service,
                base_url,
            }),
        }
    }
}

#[async_trait]
impl<Request, S> Service<Request> for AnonymousMatrixService<S>
where
    Request: ruma::api::OutgoingRequest + ruma::api::OutgoingNonAuthRequest + Send,
    <Request as ruma::api::OutgoingRequest>::EndpointError: 'static,
    S: Service<http::Request<Vec<u8>>, Response=http::Response<Vec<u8>>, Error=anyhow::Error> + Send + Sync,
{
    type Response = Request::IncomingResponse;
    type Error = MatrixLibError<Request::EndpointError>;

    async fn call(&self, request: Request) -> Result<Self::Response, Self::Error>
        where Request: 'async_trait
    {
        let http_request: http::Request<Vec<u8>> = {
            let inner = self.inner.clone();
            request.try_into_http_request(&inner.deref().base_url, None)?
        };

        let http_response = self.inner.http_service.call(http_request).await?;

        Ok(http_response.try_into()?)
    }

}



#[derive(Clone, Debug, serde::Deserialize, Eq, Hash, PartialEq, serde::Serialize)]
pub struct Session {
    pub base_url: String,
    /// The user the access token was issued for.
    pub user_id: String,
    /// The access token used for this session.
    pub access_token: String,
    /// The ID of the client device
    pub device_id: String,
}

#[derive(Clone, Debug)]
pub struct MatrixService<S> {
    inner: Arc<InnerMatrixService<S>>,
}
#[derive(Debug)]
struct InnerMatrixService<S> {
    http_service: S,
    base_url: String,
    access_token: String,
}

impl<S> MatrixService<S>
where
    S: Service<http::Request<Vec<u8>>, Response=http::Response<Vec<u8>>, Error=anyhow::Error>,
{
    pub fn new(http_service: S, base_url: String, access_token: String) -> MatrixService<S> {
        Self {
            inner: Arc::new(InnerMatrixService {
                http_service,
                base_url,
                access_token,
            }),
        }
    }
}

#[async_trait]
impl<Request, S> Service<Request> for MatrixService<S>
where
    Request: ruma::api::OutgoingRequest + Send,
    <Request as ruma::api::OutgoingRequest>::EndpointError: 'static,
    S: Service<http::Request<Vec<u8>>, Response=http::Response<Vec<u8>>, Error=anyhow::Error> + Send + Sync,
{
    type Response = Request::IncomingResponse;
    type Error = MatrixLibError<Request::EndpointError>;

    async fn call(&self, request: Request) -> Result<Self::Response, Self::Error>
        where Request: 'async_trait
    {
        let http_request: http::Request<Vec<u8>> = {
            let inner = self.inner.clone();
            request.try_into_http_request(&inner.deref().base_url, Some(&inner.deref().access_token))?
        };

        let http_response = self.inner.http_service.call(http_request).await?;

        Ok(http_response.try_into()?)
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

pub async fn server_discovery<S>(http_service: S, user_id: String) -> Result<ruma::api::client::r0::session::login::DiscoveryInfo, AutoDiscoveryError>
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
    // TODO: why is this unwrap ok?
    let service = AnonymousMatrixService::new(http_service.clone(), domain.parse().unwrap());

    // 3. Make a GET request to https://hostname/.well-known/matrix/client.
    // 3c. Parse the response body as a JSON object
    let discovery_response = service.call(ruma::api::client::unversioned::discover_homeserver::Request::new()).await;

    let discovery_info = match discovery_response {
        // error on serializing into http request
        Err(MatrixLibError::IntoHttpError(error)) =>
            Err(AutoDiscoveryError::FailPrompt(format!("{}", error))),
        // 3a. If the returned status code is 404, then IGNORE.
        Err(MatrixLibError::FromHttpResponseError(error)) => {
            match error {
                ruma::api::error::FromHttpResponseError::Deserialization(source_error) =>
                    Err(AutoDiscoveryError::FailPrompt(format!("{}", source_error))),
                ruma::api::error::FromHttpResponseError::Http(server_error) => match server_error {
                    ruma::api::error::ServerError::Known(error) => {
                        if error.status_code == http::StatusCode::NOT_FOUND {
                            Err(AutoDiscoveryError::Ignore)
                        }
                        // 3b. If the returned status code is not 200, or the response body is empty, then FAIL_PROMPT.
                        else {
                            Err(AutoDiscoveryError::FailPrompt(format!("{}", error)))
                        }
                    }
                    // this is a deserialization error of the endpoint error
                    ruma::api::error::ServerError::Unknown(error) =>
                        Err(AutoDiscoveryError::FailPrompt(format!("{}", error))),
                },
                _ => Err(AutoDiscoveryError::FailPrompt("FromHttpResponseError has gained an unhandeld variant".to_string())),
            }
        },

        // // 3b. If the returned status code is not 200, or the response body is empty, then FAIL_PROMPT.
        // Err(MatrixLibError::Http(error_response)) => Err(AutoDiscoveryError::FailPrompt(format!("{}", error_response.status()))),
        // 3ci. If the content cannot be parsed, then FAIL_PROMPT.
        // 3di. If this value is not provided, then FAIL_PROMPT.
        // Err(MatrixLibError::Deserialization(source_error)) => Err(AutoDiscoveryError::FailPrompt(format!("{}", source_error))),
        Err(MatrixLibError::HttpService(source_error)) => Err(AutoDiscoveryError::FailPrompt(format!("{}", source_error))),
        // TODO: those types are the same, however they're deeply disconnected types in ruma
        Ok(discovery_response) => Ok(ruma::api::client::r0::session::login::DiscoveryInfo {
            homeserver: ruma::api::client::r0::session::login::HomeserverInfo {
                base_url: discovery_response.homeserver.base_url,
            },
            identity_server: discovery_response.identity_server.map(|identity_server|
                ruma::api::client::r0::session::login::IdentityServerInfo {
                    base_url: identity_server.base_url,
                }),
        }),
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
    let _ = service.call(ruma::api::client::unversioned::get_supported_versions::Request::new()).await
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
        let identity_status_response = service.call(identity_status::Request).await;

        identity_status_response
            .and(Ok(discovery_info))
            .map_err(|error| AutoDiscoveryError::FailError(format!("{}", error)))
    }
    else {
        Ok(discovery_info)
    }
}


// TODO: try out a Paging API


#[cfg(test)]
mod tests {
    use super::Service;

    // TODO: move to unit test
    async fn test_version_service() {
        let server_uri = http::Uri::from_static("https://ayuthay.wolkenplanet.de");

        let service = super::AnonymousMatrixService::new(super::http_services::ReqwestService::new(), server_uri.clone());

        // TODO: VersionRequest runs into an infinite recursion loop when /_synapse is not yet activated in nginx
        let version_request = super::VersionRequest;
        let version_response = service.call(version_request).await.unwrap();
        println!("{:?}", version_response);
    }

    #[test]
    fn run() {
        smol::run(async {
            test_version_service().await
        });
    }
}

