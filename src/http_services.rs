use async_trait::async_trait;
use std::convert::TryInto;

use crate::Service;

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


#[derive(Clone, Debug)]
pub struct ReqwestService {
    client: reqwest::Client,
}
impl ReqwestService {
    pub fn new() -> ReqwestService {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Service<http::Request<Vec<u8>>> for ReqwestService {
    type Response = http::Response<Vec<u8>>;
    type Error = anyhow::Error;

    async fn call(&self, http_request: http::Request<Vec<u8>>) -> Result<http::Response<Vec<u8>>, anyhow::Error> {
        println!("http request: {:?}", http_request);
        let reqwest_request: reqwest::Request = http_request.try_into()?;
        let reqwest_response = self.client.execute(reqwest_request).await?;
        let mut http_response = http::Response::new(vec![]);
        *http_response.status_mut() = reqwest_response.status();
        *http_response.headers_mut() = reqwest_response.headers().clone();
        let body = reqwest_response.bytes().await?;
        *http_response.body_mut() = body.to_vec();

        println!("received http response: {:?}", http_response);
        println!("decoded body: {:?}", std::str::from_utf8(http_response.body()).unwrap());
        Ok(http_response)
    }
}

