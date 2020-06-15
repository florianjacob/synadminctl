use std::io::Write;
use std::str::FromStr;
use synadminctl::{Channel, Session, IdentifierType};
use synadminctl::AnonymousChannel;
use structopt::StructOpt;
use std::convert::TryInto;

fn prompt_cleartext(query: &str) -> String{
    print!("{}: ", query);
    std::io::stdout().flush().unwrap();
    let mut reply = String::new();
    std::io::stdin().read_line(&mut reply).unwrap();
    String::from(reply.trim())
}

fn from_http_to_minreq_request(http_request: http::Request<Vec<u8>>) -> minreq::Request {
    let minreq_method = match http_request.method() {
        &http::Method::GET => minreq::Method::Get,
        &http::Method::HEAD => minreq::Method::Head,
        &http::Method::POST => minreq::Method::Post,
        &http::Method::PUT => minreq::Method::Put,
        &http::Method::DELETE => minreq::Method::Delete,
        &http::Method::CONNECT => minreq::Method::Connect,
        &http::Method::OPTIONS => minreq::Method::Options,
        &http::Method::TRACE => minreq::Method::Trace,
        &http::Method::PATCH => minreq::Method::Patch,
        method @ _ => minreq::Method::Custom(method.as_str().to_string()),
    };
    let minreq_request = minreq::Request::new(minreq_method, http_request.uri().to_string())
        .with_body(http_request.body().as_slice());
    let minreq_request = http_request.headers().iter()
        .fold(minreq_request, |acc, (key, value)| acc.with_header(key.as_str(), value.to_str().unwrap()));

    minreq_request
}

fn from_minreq_to_http_response(minreq_response: minreq::Response) -> http::Response<Vec<u8>> {
    let mut http_response = http::Response::new(Vec::from(minreq_response.as_bytes()));
    let http_status_code = http::StatusCode::from_u16(minreq_response.status_code.try_into().unwrap()).unwrap();
    *http_response.status_mut() = http_status_code;
    http_response.headers_mut().extend(
        minreq_response.headers.iter()
        .map(|(key, value)|
             (http::header::HeaderName::from_str(key).unwrap(),
             http::header::HeaderValue::from_str(value).unwrap())
             ));
    http_response
}

fn minreq_call(http_request: http::Request<Vec<u8>>) -> Result<http::Response<Vec<u8>>, anyhow::Error> {
    let minreq_request = from_http_to_minreq_request(http_request);
    let minreq_response = minreq_request.send()?;
    let http_response = from_minreq_to_http_response(minreq_response);

    Ok(http_response)
}

// in an async program, this would be an async function
fn reqwest_call(client: reqwest::Client, http_request: http::Request<Vec<u8>>) -> Result<http::Response<Vec<u8>>, anyhow::Error> {
    let reqwest_request: reqwest::Request = http_request.try_into()?;
    // in an async program, this would await?ed
    let reqwest_response = futures::executor::block_on(client.execute(reqwest_request))?;
    let mut http_response = http::Response::new(vec![]);
    *http_response.status_mut() = reqwest_response.status();
    *http_response.headers_mut() = reqwest_response.headers().clone();
    // in an async program, this would await?ed
    let body = futures::executor::block_on(reqwest_response.bytes())?;
    *http_response.body_mut() = body.to_vec();

    Ok(http_response)
}
fn reqwest_blocking_call(client: reqwest::blocking::Client, http_request: http::Request<Vec<u8>>) -> Result<http::Response<Vec<u8>>, anyhow::Error> {
    let reqwest_request: reqwest::blocking::Request = http_request.try_into()?;
    let reqwest_response = client.execute(reqwest_request)?;
    let mut http_response = http::Response::new(vec![]);
    *http_response.status_mut() = reqwest_response.status();
    *http_response.headers_mut() = reqwest_response.headers().clone();
    let body = reqwest_response.bytes()?;
    *http_response.body_mut() = body.to_vec();
    Ok(http_response)
}


fn load_session() -> Result<synadminctl::Session, anyhow::Error> {
    let file = std::fs::File::open("session.ron")?;
    let reader = std::io::BufReader::new(file);
    let session = ron::de::from_reader(reader)?;
    Ok(session)
}

fn store_session(session: synadminctl::Session) -> Result<(), anyhow::Error> {
    let file = std::fs::File::create("session.ron")?;
    let mut buffer = std::io::BufWriter::new(file);
    Ok(write!(
        &mut buffer,
        "{}",
        ron::ser::to_string_pretty(&session, ron::ser::PrettyConfig::default())?
    )?)
}

// TODO: move to lib.rs as an actual unit test
fn test_version() {
    let server_uri = http::Uri::from_static("https://ayuthay.wolkenplanet.de");

    let channel = AnonymousChannel::new(server_uri.clone());

    // TODO: VersionRequest runs into an infinite recursion loop when /_synapse is not yet
    // activated in nginx
    let version_request = synadminctl::VersionRequest;
    let (channel, http_request) = channel.send(version_request);
    let http_response = minreq_call(http_request).unwrap();
    let (channel, version_response) = channel.recv(http_response);
    println!("{:?}", version_response.unwrap());
}

fn autodiscover(user_id: String) -> Result<synadminctl::DiscoveryInfo, synadminctl::AutoDiscoveryError> {
    let (discovery, http_request) = synadminctl::ServerDiscovery::send(user_id)?;

    let http_response = minreq_call(http_request)
        .map_err(|error| synadminctl::AutoDiscoveryError::FailPrompt(format!("{}", error)))?;
    let (discovery, http_request) = discovery.recv(http_response)?;

    let http_response = minreq_call(http_request)
        .map_err(|error| synadminctl::AutoDiscoveryError::FailError(format!("{}", error)))?;
    discovery.validate_homeserver(http_response)?.or_else(|(discovery, http_request)| {
        let http_response = minreq_call(http_request)
            .map_err(|error| synadminctl::AutoDiscoveryError::FailError(format!("{}", error)))?;
        discovery.validate_identity_server(http_response)
    })
}

#[derive(StructOpt)]
#[structopt(about = "synapse admin command-line interface")]
enum Opt {
    Version,
    IsAdmin {
        #[structopt(long)]
        user_id: String,
    },
    CreateModifyAccount {
        #[structopt(long)]
        user_id: String,
    },
    PurgeRoom {
        #[structopt(long)]
        room_id: String,
    },
    PasswordReset {
        #[structopt(long)]
        user_id: String,
        #[structopt(long)]
        logout_devices: bool,
    },
}

fn main() {
    let opt = Opt::from_args();

    let session = if let Ok(session) = load_session() {
        session
    } else {
        let initial_device_display_name = Some(format!("Synadminctl on {}", hostname::get().unwrap().into_string().unwrap()));
        // let device id be generated by the homeserver
        let device_id = None;

        let username = prompt_cleartext("username");

        // could also prompt on stderr, should I?
        let password = rpassword::prompt_password_stdout("password: ").unwrap();

        let discovery_info = match autodiscover(username.clone()) {
            Ok(discovery_info) => discovery_info,
            Err(synadminctl::AutoDiscoveryError::Prompt) => {
                let base_url = prompt_cleartext("homeserver url");
                synadminctl::DiscoveryInfo {
                    homeserver: synadminctl::HomeserverInfo { base_url },
                    identity_server: None,
                }
            },
            Err(synadminctl::AutoDiscoveryError::FailPrompt(reason)) => {
                eprintln!("Autodiscovery returned an error: {}", reason);
                let base_url = prompt_cleartext("homeserver url");
                synadminctl::DiscoveryInfo {
                    homeserver: synadminctl::HomeserverInfo { base_url },
                    identity_server: None,
                }
            },
            Err(synadminctl::AutoDiscoveryError::FailError(reason)) => {
                eprintln!("Autodiscovery returned an unrecoverable error: {}", reason);
                return;
            },
            // TODO: this should not be needed here
            Err(synadminctl::AutoDiscoveryError::Ignore) => {
                unreachable!();
            },
        };

        // TODO: here it's somewhat unfortunate that the url was parsed before as part from autodiscovery.
        // on the other side, however, it is not parsed if it was just entered by the user
        // TODO: unwrap
        let channel = AnonymousChannel::new(discovery_info.homeserver.base_url.parse().unwrap());
        let login_request = synadminctl::LoginRequest {
            kind: "m.login.password".to_string(),
            identifier: IdentifierType {
                kind: "m.id.user".to_string(),
                user: username,
            },
            password,
            device_id,
            initial_device_display_name,
        };
        let (channel, http_request) = channel.send(login_request);
        let http_response = minreq_call(http_request).unwrap();
        let (channel, login_response) = channel.recv(http_response);
        let login_response = login_response.unwrap();
        let discovery_info = login_response.well_known.unwrap_or(discovery_info);

        let session = Session {
            server_uri: discovery_info.homeserver.base_url,
            access_token: login_response.access_token,
            user_id: login_response.user_id,
            device_id: login_response.device_id,
        };


        store_session(session.clone()).unwrap();
        session
    };


    // TODO: also use the other stuff from DiscoveryInfo?
    // TODO: hand Session to constructor?
    let channel = Channel::new(session.server_uri.parse().unwrap(), session.access_token);

    match opt {
        Opt::Version => {
            let version_request = synadminctl::VersionRequest;
            let (channel, http_request) = channel.send(version_request);
            let http_response = minreq_call(http_request).unwrap();
            let (channel, version_response) = channel.recv(http_response);
            println!("{:?}", version_response.unwrap());
        },
        Opt::IsAdmin { user_id } => {
            let is_admin_request = synadminctl::IsAdminRequest {
                user_id: "@florian:wolkenplanet.de".to_string(),
            };
            let (channel, http_request) = channel.send(is_admin_request);
            let http_response = minreq_call(http_request).unwrap();
            let (channel, is_admin_response) = channel.recv(http_response);
            println!("{:?}", is_admin_response.unwrap());
        },
        Opt::CreateModifyAccount { user_id } => {
            println!("new user creation");
            let user_id = prompt_cleartext("matrix id");
            let password = prompt_cleartext("password");
            let displayname = prompt_cleartext("displayname");
            let mail_address = prompt_cleartext("mail address");

            let threepids = vec![synadminctl::Threepid {
                medium: "email".to_string(),
                address: mail_address,
            }];

            let create_account_request = synadminctl::CreateModifyAccountRequest {
                user_id: user_id,
                password: password,
                displayname: Some(displayname),
                threepids: Some(threepids),
                avatar_url: None,
                admin: None,
                deactivated: None,
            };
            println!("{:?}", create_account_request);
            let (channel, http_request) = channel.send(create_account_request);
            let http_response = minreq_call(http_request).unwrap();
            let (channel, create_account_response) = channel.recv(http_response);
            println!("{:?}", create_account_response);

        },
        Opt::PurgeRoom { room_id } => {
            println!("room purging");

            let purge_room_request = synadminctl::PurgeRoomRequest {
                room_id: room_id,
            };
            let (channel, http_request) = channel.send(purge_room_request);
            let http_response = minreq_call(http_request).unwrap();
            let (channel, purge_room_response) = channel.recv(http_response);
            println!("{:?}", purge_room_response);
        },
        Opt::PasswordReset { user_id, logout_devices } => {
            // could also prompt on stderr, should I?
            // TODO: option for random generation
            let new_password = rpassword::prompt_password_stdout("new password: ").unwrap();

            let password_reset_request = synadminctl::PasswordResetRequest {
                user_id,
                new_password,
                logout_devices: Some(logout_devices),
            };
            let (channel, http_request) = channel.send(password_reset_request);
            let http_response = minreq_call(http_request).unwrap();
            let (channel, password_reset_response) = channel.recv(http_response);
            println!("{:?}", password_reset_response);

        },
    }
}
