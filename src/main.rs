use std::io::Write;
use synadminctl::{Session, IdentifierType, Service};
use structopt::StructOpt;
use std::convert::TryInto;
use smol::blocking;

use async_trait::async_trait;

fn prompt_cleartext(query: &str) -> String{
    print!("{}: ", query);
    std::io::stdout().flush().unwrap();
    let mut reply = String::new();
    std::io::stdin().read_line(&mut reply).unwrap();
    String::from(reply.trim())
}

// TODO: surf service: https://github.com/stjepang/smol/blob/master/examples/other-runtimes.rs
// #[derive(Clone)]
// struct SurfService {
//     client: surf::Client<?>,
// }
// impl SurfService {
//     fn new() -> SurfService {
//         Self {
//             client: surf::Client::new(),
//         }
//     }
// }

// #[async_trait]
// impl synadminctl::Service<http::Request<Vec<u8>>> for SurfService {
//     type Response = http::Response<Vec<u8>>;
//     type Error = anyhow::Error;

//     async fn call(&self, http_request: http::Request<Vec<u8>>) -> Result<http::Response<Vec<u8>>, anyhow::Error> {
//         unimplemented!();
//         // TODO: is there a conversion between http::Request and http_types::Request?
//         // let surf_request = http_request.try_into()?;
//         // let surf_response = self.client.execute(reqwest_request).await?;
//         // let mut http_response = http::Response::new(vec![]);
//         // *http_response.status_mut() = surf_response.status();
//         // *http_response.headers_mut() = surf_response.headers().clone();
//         // let body = surf_response.bytes().await?;
//         // *http_response.body_mut() = body.to_vec();

//         // Ok(http_response)
//     }
// }


// TODO: this does not work out, seemingly because Vec<u8> does not implement hyper::HttpBody
// #[derive(Clone)]
// struct HyperService {
//     // this does still switch between http and https, depending on the server uri
//     client: hyper::Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>,
// }
// impl HyperService {
//     fn new() -> HyperService {
//         let https = hyper_tls::HttpsConnector::new();
//         let client = hyper::Client::builder().build::<_, Vec<u8>>>(https);
//         Self {
//             client,
//         }
//     }
// }

// #[async_trait]
// impl synadminctl::Service<http::Request<Vec<u8>>> for HyperService {
//     type Response = http::Response<Vec<u8>>;
//     type Error = anyhow::Error;

//     async fn call(&self, http_request: http::Request<Vec<u8>>) -> Result<http::Response<Vec<u8>>, anyhow::Error> {
//         let hyper_request: hyper::Request<Vec<u8>> = http_request.try_into()?;

//         let hyper_response = self.client.request(hyper_request).await?;

//         let mut http_response = http::Response::new(vec![]);
//         *http_response.status_mut() = hyper_response.status();
//         *http_response.headers_mut() = hyper_response.headers().clone();
//         let body = hyper_response.body().collect().await?;
//         *http_response.body_mut() = body.to_vec();

//         Ok(http_response)
//     }
// }


#[derive(Clone)]
struct ReqwestService {
    client: reqwest::Client,
}
impl ReqwestService {
    fn new() -> ReqwestService {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl synadminctl::Service<http::Request<Vec<u8>>> for ReqwestService {
    type Response = http::Response<Vec<u8>>;
    type Error = anyhow::Error;

    async fn call(&self, http_request: http::Request<Vec<u8>>) -> Result<http::Response<Vec<u8>>, anyhow::Error> {
        let reqwest_request: reqwest::Request = http_request.try_into()?;
        let reqwest_response = self.client.execute(reqwest_request).await?;
        let mut http_response = http::Response::new(vec![]);
        *http_response.status_mut() = reqwest_response.status();
        *http_response.headers_mut() = reqwest_response.headers().clone();
        let body = reqwest_response.bytes().await?;
        *http_response.body_mut() = body.to_vec();

        Ok(http_response)
    }
}


fn load_session() -> Result<synadminctl::Session, anyhow::Error> {
    let file = std::fs::File::open("session.ron")?;
    let reader = std::io::BufReader::new(file);
    let session = ron::de::from_reader(reader)?;
    Ok(session)
}

fn store_session(session: synadminctl::Session) -> Result<Session, anyhow::Error> {
    let serialized = ron::ser::to_string_pretty(&session, ron::ser::PrettyConfig::default())?;

    let file = std::fs::File::create("session.ron")?;
    let mut buffer = std::io::BufWriter::new(file);
    write!(&mut buffer, "{}", serialized)?;
    Ok(session)
}

// TODO: move to unit test
async fn test_version_service() {
    let server_uri = http::Uri::from_static("https://ayuthay.wolkenplanet.de");

    let service = synadminctl::AnonymousMatrixService::new(ReqwestService::new(), server_uri.clone());

    // TODO: VersionRequest runs into an infinite recursion loop when /_synapse is not yet
    // activated in nginx
    let version_request = synadminctl::VersionRequest;
    let version_response = service.call(version_request).await.unwrap();
    println!("{:?}", version_response);
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

fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();
    let http_service = ReqwestService::new();

    smol::run(async {
        let session = if let Ok(session) = blocking!(load_session()) {
            session
        } else {
            let initial_device_display_name = Some(format!("Synadminctl on {}", hostname::get().unwrap().into_string().unwrap()));
            // let device id be generated by the homeserver
            let device_id = None;

            let username = blocking!(prompt_cleartext("username"));

            // could also prompt on stderr, should I?
            let password = blocking!(rpassword::prompt_password_stdout("password: "))?;

            let discovery_info = match dbg!(synadminctl::server_discovery(http_service.clone(), username.clone()).await) {
                Ok(discovery_info) => discovery_info,
                Err(synadminctl::AutoDiscoveryError::Prompt) => {
                    let base_url = blocking!(prompt_cleartext("homeserver url"));
                    synadminctl::DiscoveryInfo {
                        homeserver: synadminctl::HomeserverInfo { base_url },
                        identity_server: None,
                    }
                },
                Err(synadminctl::AutoDiscoveryError::FailPrompt(reason)) => {
                    eprintln!("Autodiscovery returned an error: {}", reason);
                    let base_url = blocking!(prompt_cleartext("homeserver url"));
                    synadminctl::DiscoveryInfo {
                        homeserver: synadminctl::HomeserverInfo { base_url },
                        identity_server: None,
                    }
                },
                Err(synadminctl::AutoDiscoveryError::FailError(reason)) => {
                    eprintln!("Autodiscovery returned an unrecoverable error: {}", reason);
                    panic!("Autodiscovery returned an unrecoverable error");
                },
                // TODO: this should not be needed here
                Err(synadminctl::AutoDiscoveryError::Ignore) => {
                    unreachable!();
                },
            };

            // TODO: here it's somewhat unfortunate that the url was parsed before as part from autodiscovery.
            // on the other side, however, it is not parsed if it was just entered by the user
            // TODO: unwrap
            let service = synadminctl::AnonymousMatrixService::new(http_service.clone(), discovery_info.homeserver.base_url.parse().unwrap());
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
            let login_response = service.call(login_request).await?;
            let discovery_info = login_response.well_known.unwrap_or(discovery_info);

            let session = Session {
                server_uri: discovery_info.homeserver.base_url,
                access_token: login_response.access_token,
                user_id: login_response.user_id,
                device_id: login_response.device_id,
            };


            blocking!(store_session(session.clone()))?
        };


        // TODO: also use the other stuff from DiscoveryInfo?
        // TODO: hand Session to constructor?
        let service = synadminctl::MatrixService::new(http_service.clone(), session.server_uri.parse().unwrap(), session.access_token);

        let result = match opt {
            Opt::Version => {
                let version_request = synadminctl::VersionRequest;
                let version_response = service.call(version_request).await?;
                println!("{:?}", version_response);
                Ok(())
            },
            Opt::IsAdmin { user_id } => {
                let is_admin_request = synadminctl::IsAdminRequest {
                    user_id,
                };
                let is_admin_response = service.call(is_admin_request).await?;
                println!("{:?}", is_admin_response);
                Ok(())
            },
            Opt::CreateModifyAccount { user_id } => {
                println!("new user creation");
                let password = blocking!(prompt_cleartext("password"));
                let displayname = blocking!(prompt_cleartext("displayname"));
                let mail_address = blocking!(prompt_cleartext("mail address"));

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
                let create_account_response = service.call(create_account_request).await?;
                println!("{:?}", create_account_response);
                Ok(())
            },
            Opt::PurgeRoom { room_id } => {
                println!("room purging");

                let purge_room_request = synadminctl::PurgeRoomRequest {
                    room_id: room_id,
                };
                let purge_room_response = service.call(purge_room_request).await?;
                println!("{:?}", purge_room_response);
                Ok(())
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
                let password_reset_response = service.call(password_reset_request).await?;
                println!("{:?}", password_reset_response);
                Ok(())
            },
        };
        result
    })
}
