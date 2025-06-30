use reqwest::Client;
use serde::Deserialize;
use serde_json::{Value, json};
use url::form_urlencoded;
use vercel_runtime::{Body, Error, Request, Response, StatusCode, run};

#[derive(Deserialize)]
struct BalanceRequest {
    address: String,
}

pub async fn handler(req: Request) -> Result<Response<Body>, Error> {
    let method = req.method().as_str();

    let address = match method {
        "GET" => {
            let query = req.uri().query().unwrap_or("");
            let params: Vec<(String, String)> = form_urlencoded::parse(query.as_bytes())
                .into_owned()
                .collect();

            params
                .iter()
                .find(|(k, _)| k == "address")
                .map(|(_, v)| v.clone())
        }
        "POST" => {
            let body = match req.body() {
                Body::Text(text) => text.clone(),
                Body::Binary(bytes) => String::from_utf8_lossy(bytes).to_string(),
                Body::Empty => return Ok(error_response("Empty body", StatusCode::BAD_REQUEST)),
            };

            match serde_json::from_str::<BalanceRequest>(&body) {
                Ok(data) => Some(data.address),
                Err(_) => return Ok(error_response("Invalid JSON", StatusCode::BAD_REQUEST)),
            }
        }
        _ => {
            return Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .body("Method Not Allowed".into())?);
        }
    };

    let address = match address {
        Some(a) => a,
        None => return Ok(error_response("Missing address", StatusCode::BAD_REQUEST)),
    };

    let lamports = match fetch_balance(&address).await {
        Ok(balance) => balance,
        Err(_) => {
            return Ok(error_response(
                "Failed to get balance",
                StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };

    let res_body = json!({ "lamports": lamports }).to_string();
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(res_body.into())?)
}

async fn fetch_balance(address: &str) -> Result<u64, reqwest::Error> {
    let client = Client::new();
    let solana_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getBalance",
        "params": [address]
    });

    let response: Value = client
        .post("https://api.mainnet-beta.solana.com")
        .json(&solana_request)
        .send()
        .await?
        .json()
        .await?;

    Ok(response["result"]["value"].as_u64().unwrap_or(0))
}

fn error_response(msg: &str, status: StatusCode) -> Response<Body> {
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(json!({ "error": msg }).to_string().into())
        .unwrap()
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(handler).await
}