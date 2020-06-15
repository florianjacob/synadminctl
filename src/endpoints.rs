use std::convert::TryFrom;
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;

use crate::MatrixLibError;

pub trait Endpoint:
    Into<http::Request<Vec<u8>>>
{
    /// Data returned in a successful response from the endpoint.
    type Response: TryFrom<http::Response<Vec<u8>>, Error = MatrixLibError>;

    const REQUIRES_AUTHENTICATION: bool;
}


// TODO: it's still way to go to a symmetric API for client & server…
// TODO: For the admin APIs, I could use more robust serde_json::Values & co directly
// instead of struct definitions and deserailization…
// https://docs.rs/serde_json/1.0.50/serde_json/enum.Value.html#method.get


// This should actually be an enum for all the identifier types,
// but is essentially a struct just representing m.id.user.
// at least, user cann be full Matrix ID or just the local part
#[derive(Serialize)]
pub struct IdentifierType {
    #[serde(rename = "type")]
    pub kind: String,
    pub user: String,
}

// TODO: Login Type Request
#[derive(Serialize)]
pub struct LoginRequest {
    #[serde(rename = "type")]
    pub kind: String,
    pub identifier: IdentifierType,
    // actually optional, but this is tied to m.login.password
    pub password: String,
    pub device_id: Option<String>,
    pub initial_device_display_name: Option<String>,
}

impl Endpoint for LoginRequest {
    type Response = LoginResponse;
    const REQUIRES_AUTHENTICATION: bool = false;
}

impl Into<http::Request<Vec<u8>>> for LoginRequest {
    fn into(self) -> http::Request<Vec<u8>> {
        let body = serde_json::to_vec(&self).unwrap();
        let mut http_request = http::Request::new(body);
        *http_request.uri_mut() = http::Uri::from_static("/_matrix/client/r0/login");
        *http_request.method_mut() = http::Method::POST;
        http_request
    }
}

/// Client configuration provided by the server.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DiscoveryInfo {
    #[serde(rename = "m.homeserver")]
    pub homeserver: HomeserverInfo,
    #[serde(rename = "m.identity_server")]
    pub identity_server: Option<IdentityServerInfo>,
}

/// Information about the homeserver to connect to.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HomeserverInfo {
    /// The base URL for the homeserver for client-server connections.
    pub base_url: String,
}

/// Information about the identity server to connect to.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IdentityServerInfo {
    /// The base URL for the identity server for client-server connections.
    pub base_url: String,
}



#[derive(Deserialize, Debug)]
pub struct LoginResponse {
    pub user_id: String,
    pub access_token: String,
    pub device_id: String,
    pub well_known: Option<DiscoveryInfo>,
    #[deprecated(note = "extract from user_id")]
    home_server: Option<String>,
}

impl TryFrom<http::Response<Vec<u8>>> for LoginResponse {
    type Error = MatrixLibError;

    fn try_from(resp: http::Response<Vec<u8>>) -> Result<LoginResponse, Self::Error> {
        if resp.status() == http::StatusCode::OK {
            Ok(serde_json::from_slice(resp.body())?)
        } else {
            Err(MatrixLibError::Http(resp))
        }
    }
}


#[derive(Serialize)]
pub struct VersionRequest;

impl Into<http::Request<Vec<u8>>> for VersionRequest {
    fn into(self) -> http::Request<Vec<u8>> {
        let mut http_request = http::Request::new(vec![]);
        *http_request.method_mut() = http::Method::GET;
        *http_request.uri_mut() = http::Uri::from_static("/_synapse/admin/v1/server_version");
        http_request
    }
}

impl Endpoint for VersionRequest {
    type Response = VersionResponse;
    const REQUIRES_AUTHENTICATION: bool = false;
}


#[derive(Deserialize, Debug)]
pub struct VersionResponse {
    pub server_version: String,
    pub python_version: String,
}


impl TryFrom<http::Response<Vec<u8>>> for VersionResponse {
    type Error = MatrixLibError;

    fn try_from(resp: http::Response<Vec<u8>>) -> Result<VersionResponse, Self::Error> {
        if resp.status() == http::StatusCode::OK {
            Ok(serde_json::from_slice(resp.body())?)
        } else {
            Err(MatrixLibError::Http(resp))
        }
    }
}

#[derive(Serialize)]
pub struct ClientVersionRequest;

impl Into<http::Request<Vec<u8>>> for ClientVersionRequest {
    fn into(self) -> http::Request<Vec<u8>> {
        let mut http_request = http::Request::new(vec![]);
        *http_request.method_mut() = http::Method::GET;
        *http_request.uri_mut() = http::Uri::from_static("/_matrix/client/versions");
        http_request
    }
}

impl Endpoint for ClientVersionRequest {
    type Response = ClientVersionResponse;
    const REQUIRES_AUTHENTICATION: bool = false;
}


#[derive(Deserialize, Debug)]
pub struct ClientVersionResponse {
    pub versions: Vec<String>,
    pub unstable_features: Option<BTreeMap<String, bool>>,
}


impl TryFrom<http::Response<Vec<u8>>> for ClientVersionResponse {
    type Error = MatrixLibError;

    fn try_from(resp: http::Response<Vec<u8>>) -> Result<ClientVersionResponse, Self::Error> {
        if resp.status() == http::StatusCode::OK {
            Ok(serde_json::from_slice(resp.body())?)
        } else {
            Err(MatrixLibError::Http(resp))
        }
    }
}


#[derive(Serialize)]
pub struct IdentityStatusRequest;

impl Into<http::Request<Vec<u8>>> for IdentityStatusRequest {
    fn into(self) -> http::Request<Vec<u8>> {
        let mut http_request = http::Request::new(vec![]);
        *http_request.method_mut() = http::Method::GET;
        *http_request.uri_mut() = http::Uri::from_static("/_matrix/identity/api/v1");
        http_request
    }
}

impl Endpoint for IdentityStatusRequest {
    type Response = IdentityStatusResponse;
    const REQUIRES_AUTHENTICATION: bool = false;
}


#[derive(Deserialize, Debug)]
pub struct IdentityStatusResponse;


impl TryFrom<http::Response<Vec<u8>>> for IdentityStatusResponse {
    type Error = MatrixLibError;

    fn try_from(resp: http::Response<Vec<u8>>) -> Result<IdentityStatusResponse, Self::Error> {
        if resp.status() == http::StatusCode::OK {
            Ok(IdentityStatusResponse)
        } else {
            Err(MatrixLibError::Http(resp))
        }
    }
}



#[derive(Serialize, Debug)]
pub struct PurgeRoomRequest {
    pub room_id: String,
}

impl Into<http::Request<Vec<u8>>> for PurgeRoomRequest {
    fn into(self) -> http::Request<Vec<u8>> {
        let body = serde_json::to_vec(&self).unwrap();
        let mut http_request = http::Request::new(body);
        *http_request.uri_mut() = http::Uri::from_static("/_synapse/admin/v1/purge_room");
        *http_request.method_mut() = http::Method::POST;
        http_request
    }
}


#[derive(Deserialize, Debug)]
pub struct PurgeRoomResponse {
}

impl TryFrom<http::Response<Vec<u8>>> for PurgeRoomResponse {
    type Error = MatrixLibError;

    fn try_from(resp: http::Response<Vec<u8>>) -> Result<PurgeRoomResponse, Self::Error> {
        if resp.status() == http::StatusCode::OK {
            Ok(serde_json::from_slice(resp.body())?)
        } else {
            Err(MatrixLibError::Http(resp))
        }
    }
}

impl Endpoint for PurgeRoomRequest {
    type Response = PurgeRoomResponse;
    const REQUIRES_AUTHENTICATION: bool = true;
}


#[derive(Serialize, Debug)]
pub struct Threepid {
    pub medium: String,
    pub address: String,
}

#[derive(Serialize, Debug)]
pub struct CreateModifyAccountRequest {
    // part of url
    #[serde(skip_serializing)]
    pub user_id: String,

    // TODO: password should also be optional for modify user account,
    // but that's not written in the docs. Don't care for now, I mainly want to create users.
    // -> it is, and especially when changing passwords, it has the "all device logout" semantics
    pub password: String,

    // NOTE: Server explodes if attributes are not omitted but specified as null, like the default
    // Serde case.

    // defaults to user_id, or the current value if user already exists
    #[serde(skip_serializing_if="Option::is_none")]
    pub displayname: Option<String>,
    // defaults to empty, or the current value if user already exists
    #[serde(skip_serializing_if="Option::is_none")]
    pub threepids: Option<Vec<Threepid>>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub avatar_url: Option<String>,
    // defaults to false, or the current value if user already exists
    #[serde(skip_serializing_if="Option::is_none")]
    pub admin: Option<bool>,
    // defaults to false, or the current value if user already exists
    #[serde(skip_serializing_if="Option::is_none")]
    pub deactivated: Option<bool>,
}

impl Into<http::Request<Vec<u8>>> for CreateModifyAccountRequest {
    fn into(self) -> http::Request<Vec<u8>> {
        // TODO: can this go wrong, with a valid request? Then this would need to be try_into as well
        let body = serde_json::to_vec(&self).unwrap();
        println!("{:?}", body);
        let mut http_request = http::Request::new(body);
        *http_request.method_mut() = http::Method::PUT;
        *http_request.uri_mut() = format!("/_synapse/admin/v2/users/{}", self.user_id).parse().unwrap();
        http_request
    }
}

impl Endpoint for CreateModifyAccountRequest {
    type Response = CreateModifyAccountResponse;
    const REQUIRES_AUTHENTICATION: bool = true;
}

#[derive(Deserialize, Debug)]
pub struct CreateModifyAccountResponse {
}


impl TryFrom<http::Response<Vec<u8>>> for CreateModifyAccountResponse {
    type Error = MatrixLibError;

    fn try_from(resp: http::Response<Vec<u8>>) -> Result<CreateModifyAccountResponse, Self::Error> {
        // TODO: returns 200 if account-exist-and-was-updated,
        // but 201 CREATED if a new account was created.
        if resp.status() == http::StatusCode::OK {
            Ok(serde_json::from_slice(resp.body())?)
        } else {
            Err(MatrixLibError::Http(resp))
        }
    }
}


#[derive(Serialize)]
pub struct PasswordResetRequest {
    // part of url
    #[serde(skip_serializing)]
    pub user_id: String,

    pub new_password: String,
    // whether to invalidate all access tokens, i.e. whether the password was just forgotten
    // or whether the password got compromised potentially.
    // defaults to true if not set
    #[serde(skip_serializing_if="Option::is_none")]
    pub logout_devices: Option<bool>,
}

impl Into<http::Request<Vec<u8>>> for PasswordResetRequest {
    fn into(self) -> http::Request<Vec<u8>> {
        let body = serde_json::to_vec(&self).unwrap();
        let mut http_request = http::Request::new(body);
        *http_request.uri_mut() = http::Uri::from_static("/_matrix/client/r0/login");
        *http_request.uri_mut() = format!("/_synapse/admin/v1/reset_password/{}", self.user_id).parse().unwrap();
        *http_request.method_mut() = http::Method::GET;
        http_request
    }
}

impl Endpoint for PasswordResetRequest {
    type Response = PasswordResetResponse;
    const REQUIRES_AUTHENTICATION: bool = true;
}


#[derive(Deserialize, Debug)]
pub struct PasswordResetResponse {
}


impl TryFrom<http::Response<Vec<u8>>> for PasswordResetResponse {
    type Error = MatrixLibError;

    fn try_from(resp: http::Response<Vec<u8>>) -> Result<PasswordResetResponse, Self::Error> {
        if resp.status() == http::StatusCode::OK {
            Ok(serde_json::from_slice(resp.body())?)
        } else {
            Err(MatrixLibError::Http(resp))
        }
    }
}

#[derive(Serialize, Debug)]
pub struct IsAdminRequest {
    // TODO: *this* requires not only localpart, but full matrix id
    pub user_id: String,
}

impl Endpoint for IsAdminRequest {
    type Response = IsAdminResponse;
    const REQUIRES_AUTHENTICATION: bool = true;
}

impl Into<http::Request<Vec<u8>>> for IsAdminRequest {
    fn into(self) -> http::Request<Vec<u8>> {
        let mut http_request = http::Request::new(vec![]);
        *http_request.method_mut() = http::Method::GET;
        *http_request.uri_mut() = format!("/_synapse/admin/v1/users/{}/admin", self.user_id).parse().unwrap();
        http_request
    }
}

#[derive(Deserialize, Debug)]
pub struct IsAdminResponse {
    admin: bool,
}

impl TryFrom<http::Response<Vec<u8>>> for IsAdminResponse {
    type Error = MatrixLibError;

    fn try_from(resp: http::Response<Vec<u8>>) -> Result<IsAdminResponse, Self::Error> {
        if resp.status() == http::StatusCode::OK {
            Ok(serde_json::from_slice(resp.body())?)
        } else {
            Err(MatrixLibError::Http(resp))
        }
    }
}

#[derive(Debug)]
pub struct DiscoveryRequest;

impl Endpoint for DiscoveryRequest {
    type Response = DiscoveryResponse;
    const REQUIRES_AUTHENTICATION: bool = false;
}

impl Into<http::Request<Vec<u8>>> for DiscoveryRequest {
    fn into(self) -> http::Request<Vec<u8>> {
        let mut http_request = http::Request::new(vec![]);
        *http_request.method_mut() = http::Method::GET;
        *http_request.uri_mut() = http::Uri::from_static("/.well-known/matrix/client");
        http_request
    }
}

pub enum DiscoveryResponse {
    Some(DiscoveryInfo),
    None,
}

impl TryFrom<http::Response<Vec<u8>>> for DiscoveryResponse {
    type Error = MatrixLibError;

    fn try_from(resp: http::Response<Vec<u8>>) -> Result<DiscoveryResponse, Self::Error> {
        match resp.status() {
            http::StatusCode::OK => Ok(DiscoveryResponse::Some(serde_json::from_slice(resp.body())?)),
            http::StatusCode::NOT_FOUND => Ok(DiscoveryResponse::None),
            _ => Err(MatrixLibError::Http(resp)),
        }
    }
}
