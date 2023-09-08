use std::env;
use std::net::Ipv4Addr;

use serde::Deserialize;
use serde::Serialize;
use warp::{http::Response, Filter};

use crate::ms_graph::azure;

// Request headers
// {
//     "host": "127.0.0.1:34963",
//     "x-azure-functions-hostversion": "4.24.4.4",
//     "x-azure-functions-invocationid": "83a33220-a921-460a-bec0-b3f043dcf1ff",
//     "user-agent": "Azure-Functions-Host/4.24.4.4",
//     "transfer-encoding": "chunked",
//     "traceparent": "00-bb2bd5eef54cb15a1506d0326d2489e7-d3f54c84c7034d76-00",
//     "content-type": "application/json; charset=utf-8"
// }

// Timer payload
// {
//     "Data": {
//         "timer": {
//             "Schedule":{
//                 "AdjustForDST":true
//             },
//             "ScheduleStatus":null,
//             "IsPastDue":false
//         }
//     },
//     "Metadata":{
//         "sys":{
//             "MethodName":"azure",
//             "UtcNow":"2023-09-07T11:40:45.004275Z",
//             "RandGuid":"35e6e68c-5583-436c-a277-5aec2b416ba8"
//         }
//     }
// }
/// https://learn.microsoft.com/en-us/azure/azure-functions/functions-custom-handlers#request-payload
#[derive(Debug, Serialize, Deserialize, Default)]
struct AzureInvokeRequest {
    #[serde(rename = "Data")]
    data: serde_json::Value,
    #[serde(rename = "Metadata")]
    metadata: serde_json::Value,
}

/// https://learn.microsoft.com/en-us/azure/azure-functions/functions-custom-handlers#response-payload
#[derive(Debug, Serialize, Deserialize, Default)]
struct AzureInvokeResponse {
    #[serde(rename = "Outputs")]
    outputs: Option<serde_json::Value>,
    #[serde(rename = "Logs")]
    logs: Vec<String>,
    #[serde(rename = "ReturnValue")]
    return_value: Option<serde_json::Value>,
}

impl warp::Reply for AzureInvokeResponse {
    fn into_response(self) -> warp::reply::Response {
        Response::builder()
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&self).unwrap().into())
            .unwrap()
    }
}

pub(crate) async fn start_server() {
    let routes = warp::post()
        .and(warp::path("azure"))
        .and(warp::body::bytes())
        .then(|bytes: bytes::Bytes| async move {
            let result = azure().await;
            let logs = match result {
                Ok(_) => "success".to_owned(),
                Err(e) => format!("{:?}:{}", e, e.to_string()),
            };
            AzureInvokeResponse {
                outputs: None,
                // TODO Fix logging
                logs: vec![
                    "azure".to_owned(),
                    String::from_utf8((*bytes).to_vec()).unwrap_or("no_bytes".to_owned()),
                    logs,
                ],
                return_value: None,
            }
        });

    let port_key = "FUNCTIONS_CUSTOMHANDLER_PORT";
    let port: u16 = match env::var(port_key) {
        Ok(val) => val.parse().expect("Custom Handler port is not a number!"),
        Err(_) => 3000,
    };

    warp::serve(routes).run((Ipv4Addr::LOCALHOST, port)).await
}


#[tokio::test]
async fn test_azure_route() {
    tokio::spawn(start_server());
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:3000/azure")
        .body("Hello, Azure")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
}
