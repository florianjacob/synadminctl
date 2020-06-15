use std::convert::TryFrom;
use std::convert::TryInto;
use serde::{Serialize, Deserialize};

use async_trait::async_trait;

use async_std::io::ReadExt;

// muss tatsächlich empty sein, GET und PATH sind http-Details
#[derive(Serialize)]
pub struct VersionRequest {
    // not part of the actual request
    #[serde(skip_serializing)]
    host: http_types::Url,
}

impl VersionRequest {
    pub fn new(host: http_types::Url) -> VersionRequest {
        Self {
            host: host,
        }
    }
}

// impl Into<http::Request<Vec<u8>>> for VersionRequest {
//     fn into(self) -> http::Request<Vec<u8>> {
//         // TODO: schöner mit Parts?
//         http::Request::get(http::Uri::from_static("/_synapse/admin/v1/server_version")).body(vec![]).unwrap()
//     }
// }

impl Into<http_types::Request> for VersionRequest {
    fn into(self) -> http_types::Request {
        let url = self.host.join("/_synapse/admin/v1/server_version").unwrap();
        http_types::Request::new(http_types::Method::Get, url)
    }
}


#[derive(Deserialize)]
pub struct VersionResponse {
    pub server_version: String,
    pub python_version: String,
}

// impl TryFrom<http::Response<Vec<u8>>> for VersionResponse {
//     type Error = serde_json::Error;

//     fn try_from(resp: http::Response<Vec<u8>>) -> Result<VersionResponse, Self::Error> {
//         // http resp -> serde_json -> VersionResponse
//         // fallible deserialization! das kann ein http error, ein serde error und ein
//         // VersionResponse error sein (wenn ich da mehr validieren würde als String)

//         let (parts, body) = resp.into_parts();
//         // TODO: error wenn nicht
//         // assert_eq!(self.status(), StatusCode::OK);
//         assert_eq!(parts.status, http::StatusCode::OK);
//         serde_json::from_slice(&body)
//     }
// }

impl TryFrom<http_types::Response> for VersionResponse {
    type Error = serde_json::Error;

    fn try_from(resp: http_types::Response) -> Result<VersionResponse, Self::Error> {
        assert_eq!(resp.status(), http_types::StatusCode::Ok);

        // http resp -> serde_json -> VersionResponse
        // fallible deserialization! das kann ein http error, ein serde error und ein
        // VersionResponse error sein (wenn ich da mehr validieren würde als String)

        let mut string = String::new();
        let mut body: http_types::Body = resp.into();
        async_std::task::block_on(async {
            body.read_to_string(&mut string).await.unwrap();
        });
        serde_json::from_str(&string)
    }
}



// use http::Response;
// use serde::de;

// TODO: das hier ist eigentlich ein Layer/Middleware, das eben genau einen Teil von
// Request/Response den Typ modifiziert
// fn deserialize<T>(req: Response<Vec<u8>>) -> serde_json::Result<Response<T>>
//     where for<'de> T: de::Deserialize<'de>,
// {
//     let (parts, body) = req.into_parts();
//     let body = serde_json::from_slice(&body)?;
//     Ok(Response::from_parts(parts, body))
// }

#[async_trait]
pub trait Service<Request> {
    type Response;
    type Error;

    async fn call(&mut self, _: Request) -> Result<Self::Response, Self::Error>;
}

pub struct AdminService<T> {
    http_service: T,
    access_token: String,
}


impl<T> AdminService<T>
where
    T: Service<http_types::Request,Response=http_types::Response,Error=http_types::Error> + Send,
    // T: Service<http::Request<Vec<u8>>,Response=http::Response<Vec<u8>>,Error=http::Error>,
    // T::Response = http::Response<Vec<u8>>,
    // TODO: das hier ist falsch, muss ein allgemeinerer Fehler sein der http, serde und Domäne umschließt
    // T::Error = http::Error,
{
    pub fn new(http_service: T, access_token: String) -> AdminService<T> {
        AdminService {
            http_service,
            access_token,
        }
    }
}

#[async_trait]
impl<T> Service<VersionRequest> for AdminService<T>
where
    T: Service<http_types::Request,Response=http_types::Response,Error=http_types::Error> + Send,
    // T: Service<http::Request<Vec<u8>>,Response=http::Response<Vec<u8>>,Error=http::Error>,
    // T::Response = http::Response<Vec<u8>>,
    // T::Error = http::Error,
{
    type Response = VersionResponse;
    type Error = http_types::Error;

    async fn call(&mut self, req: VersionRequest) -> Result<Self::Response, Self::Error> {
        let mut http_req: http_types::Request = req.into();
        let authorization_string = format!("Bearer {}", self.access_token);
        http_req.insert_header("Authorization", authorization_string).unwrap();
        // let mut uri_parts = self.server.clone().into_parts();
        // uri_parts.path_and_query = http_req.uri().path_and_query().cloned();
        // *http_req.uri_mut() = http::Uri::from_parts(uri_parts).unwrap();
        // TODO: hier schmeiß ich den inneren serde error weg und pack ein ok rum weil ich keinen
        // passenden Fehler hab
        let resp = self.http_service.call(http_req).await.unwrap();
        Ok(resp.try_into().unwrap())
    }
}
