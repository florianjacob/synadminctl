use std::convert::TryFrom;
use std::convert::TryInto;
use serde::{Serialize, Deserialize};
use anyhow::anyhow;
use anyhow::Context;

// TODO: bis hin zu ner symmetrischen API ist es noch weit…
// TODO: Ich könnte statt der Stuct-Definitionssatz und Deserialisierung
// auch robustere serde_json::Values & co direkt benutzen:
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
    // not part of the actual request
    #[serde(skip_serializing)]
    pub host: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub identifier: IdentifierType,
    // actually optional, but this is tied to m.login.password
    pub password: String,
    pub device_id: Option<String>,
    pub initial_device_display_name: Option<String>,
}

impl TryInto<minreq::Request> for LoginRequest {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<minreq::Request, Self::Error> {
        let body = serde_json::to_string(&self)?;
        Ok(minreq::post(self.host + "/_matrix/client/r0/login")
            .with_body(body))
    }
}

/// Client configuration provided by the server.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DiscoveryInfo {
    #[serde(rename = "m.homeserver")]
    homeserver: HomeserverInfo,
    #[serde(rename = "m.identity_server")]
    identity_server: Option<IdentityServerInfo>,
}

/// Information about the homeserver to connect to.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HomeserverInfo {
    /// The base URL for the homeserver for client-server connections.
    base_url: String,
}

/// Information about the identity server to connect to.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IdentityServerInfo {
    /// The base URL for the identity server for client-server connections.
    base_url: String,
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

impl TryFrom<minreq::Response> for LoginResponse {
    type Error = anyhow::Error;

    fn try_from(resp: minreq::Response) -> Result<LoginResponse, Self::Error> {
        let body = resp.as_str()?;
        if resp.status_code != 200 {
            return Err(anyhow!("{}: {} {}", resp.status_code, resp.reason_phrase, body));
        }
        println!("{}", body);

        serde_json::from_str(body).context("invalid json for response")
    }
}


#[derive(Serialize)]
pub struct VersionRequest {
    // not part of the actual request
    #[serde(skip_serializing)]
    host: String,
}

impl VersionRequest {
    pub fn new(host: String) -> VersionRequest {
        Self {
            host: host,
        }
    }
}

impl Into<minreq::Request> for VersionRequest {
    fn into(self) -> minreq::Request {
        minreq::get(self.host + "/_synapse/admin/v1/server_version")
    }
}


#[derive(Deserialize)]
pub struct VersionResponse {
    pub server_version: String,
    pub python_version: String,
}


impl TryFrom<minreq::Response> for VersionResponse {
    type Error = anyhow::Error;

    fn try_from(resp: minreq::Response) -> Result<VersionResponse, Self::Error> {
        let body = resp.as_str()?;
        if resp.status_code != 200 {
            return Err(anyhow!("{}: {} {}", resp.status_code, resp.reason_phrase, body));
        }

        serde_json::from_str(body).context("invalid json for response")
    }
}

#[derive(Serialize, Debug)]
pub struct PurgeRoomRequest {
    // not part of the actual request
    #[serde(skip_serializing)]
    pub host: String,

    pub room_id: String,
}

impl TryInto<minreq::Request> for PurgeRoomRequest {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<minreq::Request, Self::Error> {
        let body = serde_json::to_string(&self)?;
        Ok(minreq::post(self.host + "/_synapse/admin/v1/purge_room")
            .with_body(body))
    }
}

#[derive(Deserialize, Debug)]
pub struct PurgeRoomResponse {
}


impl TryFrom<minreq::Response> for PurgeRoomResponse {
    type Error = anyhow::Error;

    fn try_from(resp: minreq::Response) -> Result<PurgeRoomResponse, Self::Error> {
        let body = resp.as_str()?;
        if resp.status_code != 200 {
            return Err(anyhow!("{}: {} {}", resp.status_code, resp.reason_phrase, body));
        }

        // der body ist zwar leer, aber ich träume davon, das hier irgendwie zu abstrahieren
        serde_json::from_str(body).context("invalid json for response")
    }
}



#[derive(Serialize, Debug)]
pub struct Threepid {
    pub medium: String,
    pub address: String,
}

#[derive(Serialize, Debug)]
pub struct CreateModifyAccountRequest {
    // not part of the actual request
    #[serde(skip_serializing)]
    pub host: String,

    // part of url
    #[serde(skip_serializing)]
    pub user_id: String,

    // TODO: password müsste auch optional sein für modify user account, aber das steht nicht in
    // der Doku. Egal erstmal, ich will vor allem Nutzer erstellen.
    // -> ist es wohl, und hat insbesondere die passwordänderung -> all device logout semantik
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

impl Into<minreq::Request> for CreateModifyAccountRequest {
    fn into(self) -> minreq::Request {
        // TODO: kann das fehlschlagen? Dann müsste das tr_into werden
        let body = serde_json::to_string(&self).unwrap();
        println!("{}", body);
        minreq::put(self.host + "/_synapse/admin/v2/users/" + &self.user_id)
            .with_body(body)
    }
}


#[derive(Deserialize, Debug)]
pub struct CreateModifyAccountResponse {
}


impl TryFrom<minreq::Response> for CreateModifyAccountResponse {
    type Error = anyhow::Error;

    fn try_from(resp: minreq::Response) -> Result<CreateModifyAccountResponse, Self::Error> {
        let body = resp.as_str()?;
        // 200 bei account-existiert-und-update, 201 für CREATED
        // thread 'main' panicked at 'called `Result::unwrap()` on an `Err` value: 201: Created {"name": "@sebastian.friebe:dsn.tm.kit.edu", "password_hash": "$2b$12$WC7Nj.CDlb86eWP.Wtrn6.VxaO2uacWW2/LNTaMxM/9WUfxZe5l1.", "is_guest": 0, "admin": 0, "consent_version": null, "consent_server_notice_sent": null, "appservice_id": null, "creation_ts": 1584379560, "user_type": null, "deactivated": 0, "displayname": "Sebastian Friebe", "avatar_url": null, "threepids": [{"medium": "email", "address": "sebastian.friebe@kit.edu", "validated_at": 1584379560586, "added_at": 1584379560586}]}', src/main.rs:170:35
        if resp.status_code != 200 {
            return Err(anyhow!("{}: {} {}", resp.status_code, resp.reason_phrase, body));
        }

        serde_json::from_str(body).context("invalid json for response")
    }
}


#[derive(Serialize)]
pub struct PasswordResetRequest {
    // not part of the actual request
    #[serde(skip_serializing)]
    host: String,

    // part of url
    #[serde(skip_serializing)]
    user_id: String,

    new_password: String,
    // whether to invalidate all access tokens, i.e. whether the password was just forgotten
    // or whether the password got compromised potentially.
    // defaults to true if not set
    #[serde(skip_serializing_if="Option::is_none")]
    logout_devices: Option<bool>,
}

impl TryInto<minreq::Request> for PasswordResetRequest {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<minreq::Request, Self::Error> {
        let body = serde_json::to_string(&self)?;
        Ok(minreq::get(self.host + "/_synapse/admin/v1/reset_password/" + &self.user_id)
            .with_body(body))
    }
}

#[derive(Deserialize)]
pub struct PasswordResetResponse {
}


impl TryFrom<minreq::Response> for PasswordResetResponse {
    type Error = anyhow::Error;

    fn try_from(resp: minreq::Response) -> Result<PasswordResetResponse, Self::Error> {
        let body = resp.as_str()?;
        if resp.status_code != 200 {
            return Err(anyhow!("{}: {} {}", resp.status_code, resp.reason_phrase, body));
        }

        // der body ist zwar leer, aber ich träume davon, das hier irgendwie zu abstrahieren
        serde_json::from_str(body).context("invalid json for response")
    }
}

#[derive(Serialize)]
pub struct IsAdminRequest {
    // not part of the actual request
    #[serde(skip_serializing)]
    pub host: String,

    // TODO: *hier* braucht es nicht nur den localpart sondern eine volle matrix id
    pub user_id: String,
}

impl Into<minreq::Request> for IsAdminRequest {
    fn into(self) -> minreq::Request {
        minreq::get(format!("{}/_synapse/admin/v1/users/{}/admin", self.host, self.user_id))
    }
}

#[derive(Deserialize, Debug)]
pub struct IsAdminResponse {
    admin: bool,
}

impl TryFrom<minreq::Response> for IsAdminResponse {
    type Error = anyhow::Error;

    fn try_from(resp: minreq::Response) -> Result<IsAdminResponse, Self::Error> {
        let body = resp.as_str()?;
        if resp.status_code != 200 {
            return Err(anyhow!("{}: {} {}", resp.status_code, resp.reason_phrase, body));
        }

        // der body ist zwar leer, aber ich träume davon, das hier irgendwie zu abstrahieren
        serde_json::from_str(body).context("invalid json for response")
    }
}



// TODO: das hier ist eigentlich ein Layer/Middleware, das eben genau einen Teil von
// Request/Response den Typ modifiziert
// fn deserialize<T>(req: Response<Vec<u8>>) -> serde_json::Result<Response<T>>
//     where for<'de> T: de::Deserialize<'de>,
// {
//     let (parts, body) = req.into_parts();
//     let body = serde_json::from_slice(&body)?;
//     Ok(Response::from_parts(parts, body))
// }

// Request ist kein Associated Type sondern ein generic type, damit eine Service-Implementierung
// das Service-Trait für mehrere Requests implementieren kann und nicht nur für einen.
// Response und Error sind Associated Types, weil es pro Request davon je nur einen gibt
pub trait Service<Request> {
    type Response;
    type Error;

    fn call(&mut self, _: Request) -> Result<Self::Response, Self::Error>;
}

pub struct AdminService<T> {
    http_service: T,
    access_token: String,
}


impl<T> AdminService<T>
where
    T: Service<minreq::Request,Response=minreq::Response,Error=minreq::Error>,
{
    pub fn new(http_service: T, access_token: String) -> AdminService<T> {
        AdminService {
            http_service,
            access_token,
        }
    }
}

// TODO: das muss irgendwie generisch werden, ich kopier hier immer den gleichen Code
impl<T> Service<VersionRequest> for AdminService<T>
where
    T: Service<minreq::Request,Response=minreq::Response,Error=minreq::Error> + Send,
{
    type Response = VersionResponse;
    type Error = anyhow::Error;

    fn call(&mut self, req: VersionRequest) -> Result<Self::Response, Self::Error> {
        let http_req: minreq::Request = req.into();
        let authorization_string = format!("Bearer {}", self.access_token);
        let http_req = http_req.with_header("Authorization", authorization_string);
        let resp = self.http_service.call(http_req)?;
        resp.try_into()
    }
}

impl<T> Service<LoginRequest> for AdminService<T>
where
    T: Service<minreq::Request,Response=minreq::Response,Error=minreq::Error> + Send,
{
    type Response = LoginResponse;
    type Error = anyhow::Error;

    fn call(&mut self, req: LoginRequest) -> Result<Self::Response, Self::Error> {
        let http_req: minreq::Request = req.try_into()?;
        let authorization_string = format!("Bearer {}", self.access_token);
        let http_req = http_req.with_header("Authorization", authorization_string);
        let resp = self.http_service.call(http_req)?;
        resp.try_into()
    }
}

impl<T> Service<PasswordResetRequest> for AdminService<T>
where
    T: Service<minreq::Request,Response=minreq::Response,Error=minreq::Error> + Send,
{
    type Response = PasswordResetResponse;
    type Error = anyhow::Error;

    fn call(&mut self, req: PasswordResetRequest) -> Result<Self::Response, Self::Error> {
        let http_req: minreq::Request = req.try_into()?;
        let authorization_string = format!("Bearer {}", self.access_token);
        let http_req = http_req.with_header("Authorization", authorization_string);
        let resp = self.http_service.call(http_req)?;
        resp.try_into()
    }
}

impl<T> Service<IsAdminRequest> for AdminService<T>
where
    T: Service<minreq::Request,Response=minreq::Response,Error=minreq::Error> + Send,
{
    type Response = IsAdminResponse;
    type Error = anyhow::Error;

    fn call(&mut self, req: IsAdminRequest) -> Result<Self::Response, Self::Error> {
        let http_req: minreq::Request = req.into();
        let authorization_string = format!("Bearer {}", self.access_token);
        let http_req = http_req.with_header("Authorization", authorization_string);
        let resp = self.http_service.call(http_req)?;
        resp.try_into()
    }
}

impl<T> Service<PurgeRoomRequest> for AdminService<T>
where
    T: Service<minreq::Request,Response=minreq::Response,Error=minreq::Error> + Send,
{
    type Response = PurgeRoomResponse;
    type Error = anyhow::Error;

    fn call(&mut self, req: PurgeRoomRequest) -> Result<Self::Response, Self::Error> {
        let http_req: minreq::Request = req.try_into()?;
        let authorization_string = format!("Bearer {}", self.access_token);
        let http_req = http_req.with_header("Authorization", authorization_string);
        let resp = self.http_service.call(http_req)?;
        resp.try_into()
    }
}



impl<T> Service<CreateModifyAccountRequest> for AdminService<T>
where
    T: Service<minreq::Request,Response=minreq::Response,Error=minreq::Error> + Send,
{
    type Response = CreateModifyAccountResponse;
    type Error = anyhow::Error;

    fn call(&mut self, req: CreateModifyAccountRequest) -> Result<Self::Response, Self::Error> {
        let http_req: minreq::Request = req.into();
        let authorization_string = format!("Bearer {}", self.access_token);
        let http_req = http_req.with_header("Authorization", authorization_string);
        let resp = self.http_service.call(http_req)?;
        resp.try_into()
    }
}
