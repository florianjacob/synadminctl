use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Threepid {
    pub medium: ruma::thirdparty::Medium,
    pub address: String,
}


pub mod version {
    use ruma::api::ruma_api;

    ruma_api! {
        metadata: {
            description: "version endpoint",
            method: GET,
            name: "version",
            path: "/_synapse/admin/v1/server_version",
            rate_limited: false,
            requires_authentication: false,
        }

        request: {}

        response: {
            pub server_version: String,
            pub python_version: String,
        }

        // TODO: What kind of error is needed here?
        // This is probably not a general matrix status error with json body, however, some http
        // error code is highly likely and a semantical result of “no valid identity server here”
    }
}

// TODO: isn't this covered by ruma's identity-service-api?
// -> not yet, as that crate is still empty, could send a PR. Is there a deprecation flag for the metadata?
// also: might send a PR to matrix-spec first to switch to the /_matrix/identity/v2 endpoint for identity service autodiscovery
// That MSC already exists: https://github.com/matrix-org/matrix-doc/pull/2499
pub mod identity_status {
    use ruma::api::ruma_api;

    ruma_api! {
        metadata: {
            description: "identity status endpoint",
            method: GET,
            name: "version",
            path: "/_matrix/identity/api/v1",
            rate_limited: false,
            requires_authentication: false,
        }

        request: {}

        response: {}

        // TODO: What kind of error is needed here?
        // This is probably not a general matrix status error with json body, however, some http
        // error code is highly likely and a semantical result of “no valid identity server here”
    }
}

/// https://github.com/matrix-org/synapse/blob/master/docs/admin_api/user_admin_api.rst#list-accounts
pub mod list_accounts {
    use ruma::api::ruma_api;
    use serde::{Serialize, Deserialize};

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct UserDetails {
        pub name: ruma::UserId,
        // TODO: this isn't named as optional in the spec, but missing from the responses
        pub password_hash: Option<String>,
        // TODO: why not bool?
        pub is_guest: js_int::UInt,
        // TODO: why not bool?
        pub admin: js_int::UInt,
        // TODO: what is this field? It's null in the examples
        pub user_type: Option<String>,
        // TODO: why not bool?
        pub deactivated: js_int::UInt,
        pub displayname: Option<String>,
        pub avatar_url: Option<String>,
    }

    ruma_api! {
        metadata: {
            description: "list accounts endpoint",
            method: GET,
            name: "list_accounts",
            path: "/_synapse/admin/v2/users",
            rate_limited: false,
            requires_authentication: true,
        }

        request: {
            /// TODO: this should be treated as opaque, i.e. a newtype, so that only values returned from responses can be used here
            /// Offset in the returned list. Defaults to 0.
            #[serde(skip_serializing_if="Option::is_none")]
            #[ruma_api(query)]
            pub from: Option<js_int::UInt>,
            /// Maximum amount of users to return. Defaults to 100.
            #[serde(skip_serializing_if="Option::is_none")]
            #[ruma_api(query)]
            pub limit: Option<js_int::UInt>,
            /// user_id is optional and filters to only return users with user IDs that contain this value. This parameter is ignored when using the name parameter.
            #[serde(skip_serializing_if="Option::is_none")]
            #[ruma_api(query)]
            pub user_id: Option<String>,
            /// name is optional and filters to only return users with user ID localparts or displaynames that contain this value.
            #[serde(skip_serializing_if="Option::is_none")]
            #[ruma_api(query)]
            pub name: Option<String>,
            /// The parameter guests is optional and if false will exclude guest users. Defaults to true to include guest users.
            #[serde(skip_serializing_if="Option::is_none")]
            #[ruma_api(query)]
            pub guests: Option<bool>,
            /// The parameter deactivated is optional and if true will include deactivated users. Defaults to false to exclude deactivated users.
            #[serde(skip_serializing_if="Option::is_none")]
            #[ruma_api(query)]
            pub deactivated: Option<bool>,
        }

        response: {
            pub users: Vec<UserDetails>,
            /// To paginate, check for next_token and if present, call the endpoint again with from set to the value of next_token. This will return a new page.
            /// If the endpoint does not return a next_token then there are no more users to paginate through.
            pub next_token: Option<String>,
            pub total: js_int::UInt,
        }

        error: ruma::api::client::Error
    }

}

/// https://github.com/matrix-org/synapse/blob/master/docs/admin_api/rooms.md#list-room-api
pub mod list_rooms {
    use ruma::api::ruma_api;
    use serde::{Serialize, Deserialize};

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct RoomDetails {
        pub room_id: ruma::RoomId,
        pub name: Option<String>,
        pub canonical_alias: Option<ruma::RoomAliasId>,
        pub joined_members: js_int::UInt,
        pub joined_local_members: js_int::UInt,
        pub version: String,
        #[serde(deserialize_with = "ruma::serde::empty_string_as_none")]
        pub creator: Option<ruma::UserId>,
        pub encryption: Option<String>,
        pub federatable: bool,
        pub public: bool,
        // TODO: make enum
        pub join_rules: Option<String>,
        // TODO: make enum
        pub guest_access: Option<String>,
        // TODO: make enum
        pub history_visibility: Option<String>,
        pub state_events: js_int::UInt,
    }

    ruma_api! {
        metadata: {
            description: "list rooms endpoint",
            method: GET,
            name: "list_rooms",
            path: "/_synapse/admin/v1/rooms",
            rate_limited: false,
            requires_authentication: true,
        }

        request: {
            /// Offset in the returned list. Defaults to 0.
            #[serde(skip_serializing_if="Option::is_none")]
            #[ruma_api(query)]
            pub from: Option<js_int::UInt>,
            /// Maximum amount of rooms to return. Defaults to 100.
            #[serde(skip_serializing_if="Option::is_none")]
            #[ruma_api(query)]
            pub limit: Option<js_int::UInt>,
            // TODO: enum
            #[serde(skip_serializing_if="Option::is_none")]
            #[ruma_api(query)]
            pub order_by: Option<String>,
            // TODO: enum
            #[serde(skip_serializing_if="Option::is_none")]
            #[ruma_api(query)]
            pub dir: Option<String>,
            /// Filter rooms by their room name. Search term can be contained in any part of the room name. Defaults to no filtering.
            // TODO: enum
            #[serde(skip_serializing_if="Option::is_none")]
            #[ruma_api(query)]
            pub search_term: Option<String>,
        }

        response: {
            pub rooms: Vec<RoomDetails>,
            pub offset: js_int::UInt,
            pub total_rooms: js_int::UInt,
            pub next_batch: Option<js_int::UInt>,
            pub prev_batch: Option<js_int::UInt>,
        }

        error: ruma::api::client::Error
    }

}

/// https://github.com/matrix-org/synapse/blob/master/docs/admin_api/user_admin_api.rst#query-user-account
pub mod query_user {
    use ruma::api::ruma_api;

    ruma_api! {
        metadata: {
            description: "query user endpoint",
            method: GET,
            name: "query_user",
            path: "/_synapse/admin/v2/users/:user_id",
            rate_limited: false,
            requires_authentication: true,
        }

        request: {
            #[ruma_api(path)]
            pub user_id: ruma::UserId,
        }

        response: {
            pub displayname: Option<String>,
            pub threepids: Option<Vec<super::Threepid>>,
            pub avatar_url: Option<String>,
            // TODO: this is returned as int, while the doc shows a bool
            // I could convert, or investigate further whether there are other values, or fix it upstream
            pub admin: js_int::UInt,
            // TODO: this is returned as int, while the doc shows a bool
            // I could convert, or investigate further whether there are other values, or fix it upstream
            pub deactivated: js_int::UInt,
        }

        error: ruma::api::client::Error
    }
}

/// https://github.com/matrix-org/synapse/blob/master/docs/admin_api/purge_room.md
pub mod purge_room {
    use ruma::api::ruma_api;

    ruma_api! {
        metadata: {
            description: "purge room endpoint",
            method: POST,
            name: "purge_room",
            path: "/_synapse/admin/v1/purge_room",
            rate_limited: false,
            requires_authentication: true,
        }

        request: {
            pub room_id: ruma::RoomId,
        }

        response: {}

        error: ruma::api::client::Error
    }
}



/// https://github.com/matrix-org/synapse/blob/master/docs/admin_api/user_admin_api.rst#create-or-modify-account
pub mod create_modify_account {
    use ruma::api::ruma_api;

    ruma_api! {
        metadata: {
            description: "create or modify account endpoint",
            method: PUT,
            name: "create_modify_account",
            path: "/_synapse/admin/v2/users/:user_id",
            rate_limited: false,
            requires_authentication: true,
        }

        request: {
            #[ruma_api(path)]
            pub user_id: ruma::UserId,

            // TODO: password should also be optional for modify user account,
            // but that's not written in the docs. Don't care for now, I mainly want to create users.
            // -> it is, and especially when changing passwords, it has the "all device logout" semantics
            pub password: String,

            // NOTE: Server explodes if attributes are not omitted but specified as null, like the default
            // Serde case.

            // defaults to user_id, or the current value if user already exists
            // Some("") is treated as setting it to null.
            #[serde(skip_serializing_if="Option::is_none")]
            pub displayname: Option<String>,
            // defaults to empty, or the current value if user already exists
            #[serde(skip_serializing_if="Option::is_none")]
            pub threepids: Option<Vec<super::Threepid>>,
            #[serde(skip_serializing_if="Option::is_none")]
            pub avatar_url: Option<String>,
            // defaults to false, or the current value if user already exists
            #[serde(skip_serializing_if="Option::is_none")]
            pub admin: Option<bool>,
            // defaults to false, or the current value if user already exists
            #[serde(skip_serializing_if="Option::is_none")]
            pub deactivated: Option<bool>,

        }

        // TODO: this response reverse-engineered and not documented, should all of those be required?
        // Alternative: https://serde.rs/attr-flatten.html
        response: {
            pub name: ruma::UserId,
            pub password_hash: String,
            // TODO: this is not returned as bool…?
            pub is_guest: js_int::UInt,
            // TODO: this is not returned as bool…?
            pub admin: js_int::UInt,
            // TODO: not sure if this should be Option<js::UInt>
            // this is present but can be null, therefore optional
            pub consent_version: Option<String>,
            // TODO: not sure if this should be Option<js::UInt> or whatever
            // this is present but can be null, therefore optional
            pub consent_server_notice_sent: Option<String>,
            // TODO: not sure if this should be Option<js::UInt>
            // this is present but can be null, therefore optional
            pub appservice_id: Option<String>,
            pub creation_ts: js_int::UInt,
            // this is present but can be null, therefore optional
            pub user_type: Option<String>,
            // TODO: this is not returned as bool…?
            pub deactivated: js_int::UInt,
            pub displayname: Option<String>,
            // this is present but can be null, therefore optional
            pub avatar_url: Option<String>,
            pub threepids: Option<Vec<super::Threepid>>,
            // TODO: das hier sind Extrafelder bei der Threepid nebendran
            // pub validated_at: js_int::UInt,
            // pub added_at: js_int::UInt,
        }

        error: ruma::api::client::Error

        // TODO: returns 200 if account-exist-and-was-updated,
        // but 201 CREATED if a new account was created.
        // However, ruma does throw away this information.

        // TODO: Was genau hat es mit den EndpointErrors auf sich?
        // -> Ich kann da custom code mitgeben, der die Conversion von http::Response in einen in ruma
        // error eingepackten Fehlertyp baut
        // Ich brauch den error allein schon deswegen mindestens bei allen authentifizierten
        // Requests, weil ein ungültiger Login eben solch ein Error im Matrix-Standardformat ist.
        // TODO: Müsste ich hier wo auch nen tatsächlichen Error eintragen wie ruma client api
        // error, oder reicht hier überall der Void-Default?
        // TODO: ruma api serialisiert als Ok wenn status code < 400, sonst als error. Das halte ich
        // für nicht unfragwürdig, da auch den 300-Umleitungsblock mitzunehmen und zwischen z.B. 200 Ok
        // und 201 Created nicht zu unterscheiden.
    }
}


/// https://github.com/matrix-org/synapse/blob/master/docs/admin_api/user_admin_api.rst#reset-password
pub mod reset_password {
    use ruma::api::ruma_api;

    ruma_api! {
        metadata: {
            description: "password reset endpoint",
            method: POST,
            name: "reset_password",
            path: "/_synapse/admin/v1/reset_password/:user_id",
            rate_limited: false,
            requires_authentication: true,
        }

        request: {
            #[ruma_api(path)]
            pub user_id: ruma::UserId,

            pub new_password: String,
            // whether to invalidate all access tokens, i.e. whether the password was just forgotten
            // or whether the password got compromised potentially.
            // defaults to true if not set
            #[serde(skip_serializing_if="Option::is_none")]
            pub logout_devices: Option<bool>,
        }

        response: {}

        error: ruma::api::client::Error
    }
}


/// https://github.com/matrix-org/synapse/blob/master/docs/admin_api/user_admin_api.rst
pub mod user_is_admin {
    use ruma::api::ruma_api;

    ruma_api! {
        metadata: {
            description: "is admin endpoint",
            method: GET,
            name: "user_is_admin",
            path: "/_synapse/admin/v1/users/:user_id/admin",
            rate_limited: false,
            requires_authentication: true,
        }

        request: {
            #[ruma_api(path)]
            pub user_id: ruma::UserId,
        }

        response: {
            pub admin: bool,
        }

        error: ruma::api::client::Error
    }
}
