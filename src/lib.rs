use std::convert::TryFrom;
use std::convert::TryInto;
use std::marker::PhantomData;
use thiserror::Error;

pub mod endpoints;
pub use endpoints::*;


// this would be RumaClientError if this was an actual Matrix library,
// and should not contain Synadminctl-specific errors
#[derive(Error, Debug)]
pub enum MatrixLibError {
    #[error("received malformed json")]
    Deserialization(#[from] serde_json::Error),
    #[error("http response had unexpected error")]
    Http(http::Response<Vec<u8>>),
}


// state in which we wait to send a new matrix API call
pub struct Sending;
// state in which we wait to receive the network response, to convert it into a matrix API call Response
pub struct Receiving<Response> {
    _state: PhantomData<Response>,
}

// A channel to perform matrix API calls that do not require authentication
pub struct AnonymousChannel<State> {
    pub server_uri: http::Uri,
    _state: PhantomData<State>
}

impl AnonymousChannel<Sending> {
    pub fn new(server_uri: http::Uri) -> AnonymousChannel<Sending> {
        Self {
            server_uri,
            _state: PhantomData,
        }
    }

    pub fn send<Request: Endpoint>(self, request: Request) -> (AnonymousChannel<Receiving<Request::Response>>, http::Request<Vec<u8>>) {
        let mut http_request: http::Request<Vec<u8>> = request.into();
        assert!(!Request::REQUIRES_AUTHENTICATION);

        let mut server_parts = self.server_uri.clone().into_parts();
        server_parts.path_and_query = http_request.uri().path_and_query().cloned();
        *http_request.uri_mut() = http::Uri::from_parts(server_parts).unwrap();

        (AnonymousChannel { server_uri: self.server_uri, _state: PhantomData }, http_request)
    }
}


impl<Response> AnonymousChannel<Receiving<Response>>
    where Response: TryFrom<http::Response<Vec<u8>>, Error = MatrixLibError>
{
    pub fn recv(self, response: http::Response<Vec<u8>>) -> (AnonymousChannel<Sending>, Result<Response, MatrixLibError>) {
        let matrix_response = response.try_into();

        (AnonymousChannel { server_uri: self.server_uri, _state: PhantomData }, matrix_response)
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


pub struct Channel<State> {
    server_uri: http::Uri,
    authorization_header_value: http::HeaderValue,
    _state: PhantomData<State>
}

// A channel to perform matrix API calls that can require authentication
impl Channel<Sending> {
    pub fn new(server_uri: http::Uri, access_token: String) -> Channel<Sending> {
        let authorization_string = format!("Bearer {}", access_token);
        let authorization_header_value = http::HeaderValue::from_str(&authorization_string).unwrap();
        Self {
            server_uri,
            authorization_header_value,
            _state: PhantomData,
        }
    }

    pub fn send<Request: Endpoint>(self, request: Request) -> (Channel<Receiving<Request::Response>>, http::Request<Vec<u8>>) {
        let mut http_request: http::Request<Vec<u8>> = request.into();
        if Request::REQUIRES_AUTHENTICATION {
            http_request.headers_mut().insert("Authorization", self.authorization_header_value.clone());
        }

        let mut server_parts = self.server_uri.clone().into_parts();
        server_parts.path_and_query = http_request.uri().path_and_query().cloned();
        *http_request.uri_mut() = http::Uri::from_parts(server_parts).unwrap();

        (Channel { server_uri: self.server_uri, authorization_header_value: self.authorization_header_value, _state: PhantomData }, http_request)
    }
}


impl<Response> Channel<Receiving<Response>>
    where Response: TryFrom<http::Response<Vec<u8>>, Error = MatrixLibError>
{
    pub fn recv(self, response: http::Response<Vec<u8>>) -> (Channel<Sending>, Result<Response, MatrixLibError>) {
        let matrix_response = response.try_into();

        (Channel { server_uri: self.server_uri, authorization_header_value: self.authorization_header_value, _state: PhantomData }, matrix_response)
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

// state machine to perform full server autodiscovery,
// in accordance with https://matrix.org/docs/spec/client_server/latest#server-discovery
pub struct ServerDiscovery<State> {
    channel: AnonymousChannel<State>,
}

// subsequent state group to ServerDiscovery, in which the acquired DiscoveryInfo is validated
pub struct ServerDiscoveryValidation<State> {
    discovery_info: DiscoveryInfo,
    channel: AnonymousChannel<State>,
}


impl ServerDiscovery<Sending> {
    pub fn send(user_id: String) -> Result<(ServerDiscovery<Receiving<DiscoveryResponse>>, http::Request<Vec<u8>>), AutoDiscoveryError> {
        // https://matrix.org/docs/spec/client_server/latest#well-known-uri
        // 1. Extract the server name from the user's Matrix ID by splitting the Matrix ID at the first colon.
        // 2. Extract the hostname from the server name.
        // Grammar:
        // server_name = hostname [ ":" port ]
        // user_id = "@" user_id_localpart ":" server_name
        let parts: Vec<&str> = user_id.split(':').collect();
        if parts.len() != 2 || parts.len() != 3 {
            // user_id is not a full user_id but just a username or something like that,
            // so a hostname cannot be extracted
            return Err(AutoDiscoveryError::Prompt);
        }
        let hostname = parts[1];
        // 3. Make a GET request to https://hostname/.well-known/matrix/client.
        let domain = "https://".to_string() + hostname;
        let channel = AnonymousChannel::new(domain.parse().unwrap());

        // 3. Make a GET request to https://hostname/.well-known/matrix/client.
        let (channel, http_request) = channel.send(DiscoveryRequest);
        Ok((ServerDiscovery {
            channel,
        }, http_request))
    }
}

impl ServerDiscovery<Receiving<DiscoveryResponse>> {
    pub fn recv(self, http_response: http::Response<Vec<u8>>)
        -> Result<(ServerDiscoveryValidation<Receiving<ClientVersionResponse>>, http::Request<Vec<u8>>), AutoDiscoveryError>
    {
        // 3c. Parse the response body as a JSON object
        let (channel, discovery_response) = self.channel.recv(http_response);

        let discovery_info = match discovery_response {
            // 3a. If the returned status code is 404, then IGNORE.
            Ok(DiscoveryResponse::None) => Err(AutoDiscoveryError::Ignore),
            // 3b. If the returned status code is not 200, or the response body is empty, then FAIL_PROMPT.
            Err(MatrixLibError::Http(error_response)) => Err(AutoDiscoveryError::FailPrompt(format!("{}", error_response.status()))),
            // 3ci. If the content cannot be parsed, then FAIL_PROMPT.
            // 3di. If this value is not provided, then FAIL_PROMPT.
            Err(MatrixLibError::Deserialization(source_error)) => Err(AutoDiscoveryError::FailPrompt(format!("{}", source_error))),
            Ok(DiscoveryResponse::Some(discovery_info)) => Ok(discovery_info),
        };


        // this is our only autodiscovery mechanism,
        // therefore map Ignore to Prompt and possibly return
        let discovery_info = discovery_info.map_err(
            |error| if error == AutoDiscoveryError::Ignore { AutoDiscoveryError::Prompt } else { error })?;


        // 3d. Extract the base_url value from the m.homeserver property.
        //     This value is to be used as the base URL of the homeserver.
        // 3e. Validate the homeserver base URL:
        match discovery_info.homeserver.base_url.parse() {
            // 3ei. Parse it as a URL. If it is not a URL, then FAIL_ERROR.
            Err(error) => Err(AutoDiscoveryError::FailError(format!("{}", error))),
            Ok(base_url) => {
                let channel = AnonymousChannel::new(base_url);
                let (channel, http_request) = channel.send(ClientVersionRequest);
                Ok((ServerDiscoveryValidation::<Receiving<ClientVersionResponse>> {
                    discovery_info,
                    channel,
                }, http_request))
            },
        }
    }
}

impl ServerDiscoveryValidation<Receiving<ClientVersionResponse>> {
    pub fn validate_homeserver(self, http_response: http::Response<Vec<u8>>)
        -> Result<Result<DiscoveryInfo, (ServerDiscoveryValidation<Receiving<IdentityStatusResponse>>, http::Request<Vec<u8>>)>, AutoDiscoveryError>
    {
        // 3eii. Clients SHOULD validate that the URL points to a valid homeserver before accepting it
        //     by connecting to the /_matrix/client/versions endpoint,
        //     ensuring that it does not return an error,
        //     and parsing and validating that the data conforms with the expected response format.
        //     If any step in the validation fails, then FAIL_ERROR.
        //     Validation is done as a simple check against configuration errors,
        //     in order to ensure that the discovered address points to a valid homeserver.
        let (channel, version_response) = self.channel.recv(http_response);
        if let Err(error) = version_response {
            return Err(AutoDiscoveryError::FailError(format!("{}", error)));
        }

        if let Some(identity_server_info) = &self.discovery_info.identity_server {
            // 3fi. Parse it as a URL. If it is not a URL, then FAIL_ERROR.
            let base_url = identity_server_info.base_url.parse()
                .map_err(|error| AutoDiscoveryError::FailError(format!("{}", error)))?;
            let channel = AnonymousChannel::new(base_url);
            let (channel, http_request) = channel.send(IdentityStatusRequest);
            Ok(Err((ServerDiscoveryValidation::<Receiving<IdentityStatusResponse>> {
                discovery_info: self.discovery_info,
                channel,
            }, http_request)))
        }
        else {
            Ok(Ok(self.discovery_info))
        }
    }
}

impl ServerDiscoveryValidation<Receiving<IdentityStatusResponse>> {
    pub fn validate_identity_server(self, http_response: http::Response<Vec<u8>>)
        -> Result<DiscoveryInfo, AutoDiscoveryError>
    {
        // If the m.identity_server property is present, extract the base_url value for use as the
        // base URL of the identity server. Validation for this URL is done as in the step above,
        // but using /_matrix/identity/api/v1 as the endpoint to connect to. If the
        // m.identity_server property is present, but does not have a base_url value, then
        // FAIL_ERROR.

        let (channel, identity_status_response) = self.channel.recv(http_response);
        identity_status_response
            .and(Ok(self.discovery_info))
            .map_err(|error| AutoDiscoveryError::FailError(format!("{}", error)))
    }
}




// TODO: try out a Paging API, and how that is written down as State Machine API
