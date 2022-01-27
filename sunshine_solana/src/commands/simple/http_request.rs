use crate::{Error, Value};

use maplit::hashmap;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use reqwest::{Client, Method};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HttpRequest {
    pub method: Option<String>,
    pub url: Option<String>,
    pub auth_token: Option<String>,
}

impl HttpRequest {
    pub(crate) async fn run(
        &self,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        let method = match &self.method {
            Some(s) => s.clone(),
            None => match inputs.remove("method") {
                Some(Value::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("method".to_string())),
            },
        };

        let url = match &self.url {
            Some(s) => s.clone(),
            None => match inputs.remove("url") {
                Some(Value::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("url".to_string())),
            },
        };

        let auth_token = self.auth_token.as_ref();

        let method = Method::from_bytes(method.as_bytes()).map_err(|_| Error::InvalidHttpMethod)?;

        let client = Client::new();

        let mut builder = client.request(method, url);

        if let Some(auth_token) = auth_token {
            builder = builder.bearer_auth(auth_token);
        }

        let resp = builder.send().await?;

        let status = resp.status();
        if !status.is_success() {
            return Err(Error::HttpStatus(status.as_u16()));
        }

        let resp_body = resp.text().await?;

        let outputs = hashmap! {
            "resp_body".to_owned()=> Value::String(resp_body),
        };

        Ok(outputs)
    }
}
/*


Get Req
    Json
    uri


Json to Rust(./path/key)
    value

CreateMetadata
    uri

fields.url

https://github.com/seanmonstar/reqwest/blob/master/examples/json_typed.rs
https://github.com/seanmonstar/reqwest/blob/master/examples/json_dynamic.rs


https://airtable.com/appRYVa2YoZdNsVkk/api/docs#curl/table:table%201:list
GET

Key: keynH6Eh9ZN1Y2oqw

EXAMPLE REQUEST
curl https://api.airtable.com/v0/appRYVa2YoZdNsVkk/Table%201/recWNxPJAJ4qmz1On \
  -H "Authorization: Bearer keynH6Eh9ZN1Y2oqw"

EXAMPLE RESPONSE
{
    "id": "recWNxPJAJ4qmz1On",
    "fields": {
        "Url": "https://api.jsonbin.io/b/61ddc9072675917a628edc21"
    },
    "createdTime": "2022-01-27T18:44:22.000Z"
}


POST

EXAMPLE REQUEST
curl -v -X POST https://api.airtable.com/v0/appRYVa2YoZdNsVkk/Table%201 \
  -H "Authorization: Bearer keynH6Eh9ZN1Y2oqw" \
  -H "Content-Type: application/json" \
  --data '{
  "records": [
    {
      "fields": {
        "Url": "https://api.jsonbin.io/b/61ddc9072675917a628edc21"
      }
    },
    {
      "fields": {
        "Url": "https://api.jsonbin.io/b/61ddc9072675917a628edc21"
      }
    }
  ]
}'

EXAMPLE RESPONSE
{
    "records": [
        {
            "id": "recWNxPJAJ4qmz1On",
            "fields": {
                "Url": "https://api.jsonbin.io/b/61ddc9072675917a628edc21"
            },
            "createdTime": "2022-01-27T18:44:22.000Z"
        },
        {
            "id": "recWNxPJAJ4qmz1On",
            "fields": {
                "Url": "https://api.jsonbin.io/b/61ddc9072675917a628edc21"
            },
            "createdTime": "2022-01-27T18:44:22.000Z"
        }
    ]
}
*/
