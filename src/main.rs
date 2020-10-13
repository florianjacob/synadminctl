#![warn(rust_2018_idioms, missing_debug_implementations)]

use assign::assign;

use std::io::Write;
use synadminctl::{Session, Service};
use structopt::StructOpt;
use smol::unblock;
use std::convert::TryInto;


fn prompt_cleartext(query: &str) -> String {
    print!("{}: ", query);
    // TODO: unwrap, forward io error?
    std::io::stdout().flush().unwrap();
    let mut reply = String::new();
    // TODO: unwrap, forward io error?
    std::io::stdin().read_line(&mut reply).unwrap();
    String::from(reply.trim())
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


#[derive(StructOpt)]
#[structopt(about = "synapse admin command-line interface")]
enum Opt {
    Version,
    IsAdmin {
        #[structopt(long)]
        user_id: String,
    },
    QueryUser {
        #[structopt(long)]
        user_id: String,
    },
    ListJoinedRooms {
        #[structopt(long)]
        user_id: String,
    },
    CreateModifyAccount {
        #[structopt(long)]
        user_id: String,
    },
    ListAccounts {
        from: Option<js_int::UInt>,
        limit: Option<js_int::UInt>,
        // TODO: user_id, name, guests, deactivated
    },
    ListRooms {
        from: js_int::UInt,
    },
    PurgeRoom {
        #[structopt(long)]
        room_id: String,
    },
    ResetPassword {
        #[structopt(long)]
        user_id: String,
        #[structopt(long)]
        logout_devices: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();
    let http_service = synadminctl::http_services::ReqwestService::new();

    smol::run(async {
        let session = if let Ok(session) = unblock!(load_session()) {
            session
        } else {
            // TODO: do a match case and print the error somehow, and differentiate between
            // „file not found“ and other errors like permission denied or session.ron file has wrong format
            println!("Initial Login:");
            let initial_device_display_name = format!("Synadminctl on {}", hostname::get().unwrap().into_string().unwrap());

            let username = unblock!(prompt_cleartext("username"));

            // could also prompt on stderr, should I?
            let password = unblock!(rpassword::prompt_password_stdout("password: "))?;

            // TODO: in addition, I should do a basic VersionResponse check to see whether the
            // Admin API is available and configured, i.e. /_synapse might not be available
            let discovery_info = match synadminctl::server_discovery(http_service.clone(), username.clone()).await {
                Ok(discovery_info) => discovery_info,
                Err(synadminctl::AutoDiscoveryError::Prompt) => {
                    let base_url = unblock!(prompt_cleartext("homeserver url"));
                    ruma::api::client::r0::session::login::DiscoveryInfo {
                        homeserver: ruma::api::client::r0::session::login::HomeserverInfo { base_url },
                        identity_server: None,
                    }
                },
                Err(synadminctl::AutoDiscoveryError::FailPrompt(reason)) => {
                    eprintln!("Autodiscovery returned an error: {}", reason);
                    let base_url = unblock!(prompt_cleartext("homeserver url"));
                    ruma::api::client::r0::session::login::DiscoveryInfo {
                        homeserver: ruma::api::client::r0::session::login::HomeserverInfo { base_url },
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

            let service = synadminctl::AnonymousMatrixService::new(http_service.clone(), discovery_info.homeserver.base_url.clone());
            let mut request = ruma::api::client::r0::session::login::Request::new(
                ruma::api::client::r0::session::login::UserInfo::MatrixId(&username),
                ruma::api::client::r0::session::login::LoginInfo::Password { password: &password },
            );
            request.initial_device_display_name = Some(&initial_device_display_name);
            let response = service.call(request).await?;
            let discovery_info = response.well_known.unwrap_or(discovery_info);

            let session = Session {
                base_url: discovery_info.homeserver.base_url,
                access_token: response.access_token,
                user_id: response.user_id.to_string(),
                device_id: response.device_id.to_string(),
            };


            unblock!(store_session(session.clone()))?
        };


        // TODO: also use the other stuff from DiscoveryInfo?
        // TODO: hand Session to constructor?
        let service = synadminctl::MatrixService::new(http_service.clone(), session.base_url, session.access_token);

        let result = match opt {
            Opt::Version => {
                let request = synadminctl::version::Request::new();
                let response = service.call(request).await?;
                println!("{:?}", response);
                Ok(())
            },
            Opt::IsAdmin { user_id } => {
                let request = synadminctl::user_is_admin::Request::new(
                    user_id.try_into()?,
                );
                let response = service.call(request).await?;
                println!("{:?}", response);
                Ok(())
            },
            Opt::QueryUser { user_id } => {
                let request = synadminctl::query_user::Request::new(
                    user_id.try_into()?,
                );
                println!("{:?}", request);
                let response = service.call(request).await?;
                println!("{:?}", response);
                Ok(())
            },
            Opt::ListJoinedRooms { user_id } => {
                let request = synadminctl::list_joined_rooms::Request::new(
                    user_id.try_into()?,
                );
                println!("{:?}", request);
                let response = service.call(request).await?;
                println!("{:?}", response);
                Ok(())
            },
            Opt::CreateModifyAccount { user_id } => {
                println!("new user creation");
                let password = unblock!(prompt_cleartext("password"));
                let displayname = unblock!(prompt_cleartext("displayname"));
                let mail_address = unblock!(prompt_cleartext("mail address"));

                // TODO: es gibt
                // 1. setzen auf leer
                // 2. setzen auf bestimmten wert (oder mehrere)
                // 3. alten Wert so lassen wie er war.
                // -> aktuell behandle ich leer als ignorieren
                let threepids = if mail_address != "" {
                    Some(vec![synadminctl::Threepid {
                        medium: ruma::thirdparty::Medium::Email,
                        address: mail_address,
                    }])
                } else {
                    None
                };

                let request = assign!(synadminctl::create_modify_account::Request::new(user_id.try_into()?, password), {
                    displayname: Some(displayname),
                    threepids,
                });
                println!("{:?}", request);
                let response = service.call(request).await?;
                println!("{:?}", response);
                Ok(())
            },
            Opt::ListAccounts { from, limit } => {
                let request = assign!(synadminctl::list_accounts::Request::new(), {
                    from,
                    limit,
                });
                let response = service.call(request).await?;
                println!("{:#?}", response);
                Ok(())
            },
            Opt::ListRooms { from } => {
                println!("rooms from {}", from);
                let request = assign!(synadminctl::list_rooms::Request::new(), {
                    from: Some(from),
                });
                let response = service.call(request).await?;
                println!("{:#?}", response);
                Ok(())
            },
            Opt::PurgeRoom { room_id } => {
                println!("room purging");
                let request = synadminctl::purge_room::Request::new(
                    room_id.try_into()?,
                );
                let response = service.call(request).await?;
                println!("{:?}", response);
                Ok(())
            },
            Opt::ResetPassword { user_id, logout_devices } => {
                // could also prompt on stderr, should I?
                // TODO: option for random generation
                let new_password = rpassword::prompt_password_stdout("new password: ").unwrap();

                let request = synadminctl::reset_password::Request::new(
                    user_id.try_into()?,
                    new_password,
                    Some(logout_devices),
                );
                let response = service.call(request).await?;
                println!("{:?}", response);
                Ok(())
            },
        };
        result
    })
}
