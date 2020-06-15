use synadminctl::Service;

use std::io::Write;


struct MinreqService;

// TODO: das hier verwendet minireq types als allgemeine typen, wär natürlich schöner da was
// abstraktes wie http_types hinzustellen, aber das zwingt einem zu einem async body type

impl synadminctl::Service<minreq::Request> for MinreqService
{
    type Response = minreq::Response;
    type Error = minreq::Error;

    fn call(&mut self, req: minreq::Request) -> Result<Self::Response, Self::Error> {
        req.send()
    }
}

#[derive(Clone, Debug, serde::Deserialize, Eq, Hash, PartialEq, serde::Serialize)]
pub struct Session {
    /// The access token used for this session.
    pub access_token: String,
    /// The user the access token was issued for.
    pub user_id: String,
    /// The ID of the client device
    pub device_id: String,
}

fn load_session() -> Result<Session, anyhow::Error> {
    let file = std::fs::File::open("session.ron")?;
    let reader = std::io::BufReader::new(file);
    let session = ron::de::from_reader(reader)?;
    Ok(session)
}

fn store_session(session: Session) -> Result<(), anyhow::Error> {
    let file = std::fs::File::create("session.ron")?;
    let mut buffer = std::io::BufWriter::new(file);
    Ok(write!(
        &mut buffer,
        "{}",
        ron::ser::to_string_pretty(&session, ron::ser::PrettyConfig::default())?
    )?)
}


fn main() {
    let http_service = MinreqService {};
    let access_token = "blub".to_string();

    // let server_url = "https://ayuthay.wolkenplanet.de".to_string();
    let server_url = "https://matrix.dsn.scc.kit.edu".to_string();


    // TODO: hier gibt's ein gewisses Henne-Ei-Problem mit dem access token
    let mut admin_service = synadminctl::AdminService::new(
        http_service,
        access_token,
    );
    println!("Requesting…");

    let initial_device_display_name = format!("Synadminctl on {}", hostname::get().unwrap().into_string().unwrap());

    let session = if let Ok(session) = load_session() {
        session
    } else {
        // TODO: do well-known server detection
        print!("username: ");
        std::io::stdout().flush().unwrap();
        let mut username = String::new();
        std::io::stdin().read_line(&mut username).unwrap();
        let username = String::from(username.trim());

        // could also prompt on stderr, should I?
        let password = rpassword::prompt_password_stdout("password: ").unwrap();

        let login_request = synadminctl::LoginRequest {
            host: server_url.clone(),
            kind: "m.login.password".to_string(),
            identifier: synadminctl::IdentifierType {
                kind: "m.id.user".to_string(),
                user: username.clone(),
            },
            password: password,
            // don't set device id here for now, but save them for later use together with access token
            // TODO: wobei: wenn ich eine eindeutige ID generieren kann, kann ich auch eine eigene nehmen
            // und hab ein Problem weniger, und eine sich nicht ändernde ID wenn man das access token
            // wegschmeißt.
            device_id: None,
            initial_device_display_name: Some(initial_device_display_name),
        };
        let login_response = admin_service.call(login_request).unwrap();
        let session = Session {
            access_token: login_response.access_token,
            user_id: login_response.user_id,
            device_id: login_response.device_id,
        };
        store_session(session.clone()).unwrap();
        session
    };

    // TODO: anderen login response kram mitverwenden

    let http_service = MinreqService {};
    let mut admin_service = synadminctl::AdminService::new(
        http_service,
        session.access_token,
    );

    // TODO: den access token dem Service zu geben, aber die Server URL dem Request, macht keinen
    // Sinn, gerade wenn man an mehrere Server denkt, da Token und Server URL gekoppelt sind
    // Vielleicht krieg ich das mit ureq besser hin? https://docs.rs/ureq/0.12.0/ureq/
    // Gleichzeitig aber auch wieder: Ob ein Zugriff jetzt eine Authentifizierung braucht oder
    // nicht weiß eigentlich nur der Request, der dann ein access token fordern könnte oder nicht…
    // -> auch an symmetrische API denken!



    // TODO: läuft auf matrix.dsn.scc.kit.edu in eine infinite recursion loop…?
    // -> das passiert wenn /_synapse noch nicht freigeschaltet war
    // let version_request = synadminctl::VersionRequest::new(server_url.clone());
    // let version_response = admin_service.call(version_request).unwrap();
    // println!("server_version: {}\npython_version: {}\n", version_response.server_version, version_response.python_version);

    // let is_admin_request = synadminctl::IsAdminRequest {
    //     host: server_url.clone(),
    //     user_id: "@florian:wolkenplanet.de".to_string(),
    // };
    // let is_admin_response = admin_service.call(is_admin_request).unwrap();
    // println!("{:?}", is_admin_response);


    println!("new user creation");
    print!("matrix id: ");
    std::io::stdout().flush().unwrap();
    let mut user_id = String::new();
    std::io::stdin().read_line(&mut user_id).unwrap();
    let user_id = String::from(user_id.trim());

    print!("password: ");
    std::io::stdout().flush().unwrap();
    let mut password = String::new();
    std::io::stdin().read_line(&mut password).unwrap();
    let password = String::from(password.trim());


    print!("displayname: ");
    std::io::stdout().flush().unwrap();
    let mut displayname = String::new();
    std::io::stdin().read_line(&mut displayname).unwrap();
    let displayname = String::from(displayname.trim());


    print!("mail address: ");
    std::io::stdout().flush().unwrap();
    let mut mail_address = String::new();
    std::io::stdin().read_line(&mut mail_address).unwrap();
    let mail_address = String::from(mail_address.trim());
    let threepids = vec![synadminctl::Threepid {
        medium: "email".to_string(),
        address: mail_address,
    }];

    // // TODO: das setzt zwar die E-Mail-Adresse, aber seltsamerweise nicht die Notifications by
    // // default - möglicherweise der Unterschied weil über die API und nicht über die Registrierung
    // // angelegt?
    // // -> ich hab nen Issue gemeldet und soll es selbst machen
    let create_account_request = synadminctl::CreateModifyAccountRequest {
        host: server_url.clone(),
        user_id: user_id,
        password: password,
        displayname: Some(displayname),
        threepids: Some(threepids),
        avatar_url: None,
        admin: None,
        deactivated: None,
    };
    println!("{:?}", create_account_request);
    let create_account_response = admin_service.call(create_account_request).unwrap();
    println!("{:?}", create_account_response);


    // println!("room purging");
    // print!("room_id: ");
    // std::io::stdout().flush().unwrap();
    // let mut room_id = String::new();
    // std::io::stdin().read_line(&mut room_id).unwrap();
    // let room_id = String::from(room_id.trim());

    // let purge_room_request = synadminctl::PurgeRoomRequest {
    //     host: server_url.clone(),
    //     room_id: room_id,
    // };
    // let purge_room_response = admin_service.call(purge_room_request).unwrap();
    // println!("{:?}", purge_room_response);
}
