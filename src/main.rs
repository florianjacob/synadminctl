use synadminctl::Service;

use async_trait::async_trait;

struct MinireqService;

#[async_trait]
impl synadminctl::Service<http_types::Request> for MinireqService
{
    type Response = http_types::Response;
    type Error = http_types::Error;

    async fn call(&mut self, req: http_types::Request) -> Result<Self::Response, Self::Error> {
        let mini_req = match req.method() {
            http_types::Method::Get => minireq::get(req.url()),
            http_types::Method::Head => minireq::head(req.url()),
            http_types::Method::Post => minireq::post(req.url()),
            http_types::Method::Put => minireq::put(req.url()),
            http_types::Method::Delete => minireq::delete(req.url()),
            http_types::Method::Connect => minireq::connect(req.url()),
            http_types::Method::Options => minireq::options(req.url()),
            http_types::Method::Trace => minireq::trace(req.url()),
            http_types::Method::Patch => minireq::patch(req.url()),
        };
        let mini_req = mini_req.with_header().with_body(req.into().read_to_string())

        // async_std::task::block_on(async move {
            // let stream = self.stream.lock().await;
            async_h1::client::connect(self.stream.clone(), req).await
        // })
    }
}



struct AsyncH1HttpsService {
    // stream: async_std::sync::Arc<async_std::sync::Mutex<async_native_tls::TlsStream<async_std::net::TcpStream>>>,
    stream: async_std::net::TcpStream,
    // stream: ClonableTlsStream,
}

impl AsyncH1HttpsService {
    fn new(host: String) -> AsyncH1HttpsService {
        // TODO: könnte man auch mit default headers die authorization abfrühstücken?
        let stream = async_std::task::block_on(async {
            async_std::net::TcpStream::connect(&host).await.unwrap()
            // let stream = async_std::net::TcpStream::connect(&host).await.unwrap();
            // async_native_tls::connect(&host, stream).await.unwrap()
            // let connector = async_tls::TlsConnector::default();
            // let handshake = connector.connect(&host, stream).unwrap();
            // let mut stream = handshake.await.unwrap();
        });

        Self {
            // stream: async_std::sync::Arc::new(async_std::sync::Mutex::new(stream)),
            // stream: ClonableTlsStream(async_std::sync::Arc::new(stream)),
            stream: stream,
        }
    }
}

#[async_trait]
impl synadminctl::Service<http_types::Request> for AsyncH1HttpsService
{
    type Response = http_types::Response;
    type Error = http_types::Error;

    async fn call(&mut self, req: http_types::Request) -> Result<Self::Response, Self::Error> {
        // let ptr = unsafe {
        //     self.stream.get()
        // };

        // async_std::task::block_on(async move {
            // let stream = self.stream.lock().await;
            async_h1::client::connect(self.stream.clone(), req).await
        // })
    }
}



// struct ReqwestHttpService {
//     client: reqwest::blocking::Client,
// }

// impl ReqwestHttpService {
//     fn new() -> ReqwestHttpService {
//         // TODO: man könnte auch mit default headers die authorization abfrühstücken
//         Self {
//             client: reqwest::blocking::Client::new(),
//         }
//     }
// }


// fn from_http_into_reqwest_request(req: http::Request<Vec<u8>>) -> reqwest::blocking::Request {
//     let (parts, body) = req.into_parts();
//     let http::request::Parts {
//         method,
//         uri,
//         headers,
//         ..
//     } = parts;
//     let url = reqwest::Url::parse(&uri.to_string()).unwrap();
//     let mut req = reqwest::blocking::Request::new(
//         method,
//         url,
//         );
//     *req.headers_mut() = headers;
//     *req.body_mut() = Some(body.into());
//     req
// }

// fn from_reqwest_into_http_response(resp: reqwest::blocking::Response) -> http::Response<Vec<u8>> {
//     let builder = http::Response::builder()
//         .status(resp.status())
//         .version(resp.version());
//     {
//         let mut headers = builder.headers_mut().unwrap();
//         *headers = *resp.headers();
//     }
//     builder.body(resp.into()).unwrap()
// }



// impl synadminctl::Service<http::Request<Vec<u8>>> for ReqwestHttpService
// {
//     type Response = http::Response<Vec<u8>>;
//     type Error = http::Error;

//     fn call(&mut self, req: http::Request<Vec<u8>>) -> Result<Self::Response, Self::Error> {
//         let req = from_http_into_reqwest_request(req);
//         let resp = self.client.execute(req).unwrap();
//         Ok(from_reqwest_into_http_response(resp))
//     }
// }


#[async_std::main]
async fn main() -> http_types::Result<()> {
    // let http_service = ReqwestHttpService::new();
    let http_service = AsyncH1HttpsService::new("localhost:8008".to_string());
    let access_token = "blub".to_string();
    let server_url = http_types::Url::parse("http://ayuthay.wolkenplanet.de:8008/").unwrap();

    let mut admin_service = synadminctl::AdminService::new(
        http_service,
        access_token,
    );
    println!("Requesting…");

    let request = synadminctl::VersionRequest::new(server_url);
    let response = admin_service.call(request).await.unwrap();
    println!("server_version: {}\npython_version: {}\n", response.server_version, response.python_version);

    Ok(())
}
