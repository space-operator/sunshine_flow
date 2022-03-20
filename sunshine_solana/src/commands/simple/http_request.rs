use crate::{Error, Value};

use maplit::hashmap;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use reqwest::{Client, Method};

use serde_json::Value as JsonValue;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HttpRequest {
    pub method: Option<String>,
    pub url: Option<String>,
    pub auth_token: Option<String>,
    pub json_body: Option<Option<Value>>,
    pub headers: Option<Option<Value>>,
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

        let auth_token = match self.auth_token.as_ref() {
            Some(auth_token) => Some(auth_token.clone()),
            None => match inputs.get("auth_token") {
                Some(Value::String(ref s)) => Some(s.clone()),
                None => None,
                _ => return Err(Error::ArgumentNotFound("auth_token".to_string())),
            },
        };

        let json_body = match &self.json_body {
            Some(v) => v
                .as_ref()
                .map(|v| JsonValue::try_from(v.clone()))
                .transpose()?,
            None => match inputs.remove("json_body") {
                Some(v) => Some(JsonValue::try_from(v)?),
                None => None,
            },
        };

        let headers = match &self.json_body {
            Some(v) => v
                .as_ref()
                .map(|v| JsonValue::try_from(v.clone()))
                .transpose()?,
            None => match inputs.remove("headers") {
                Some(v) => Some(JsonValue::try_from(v)?),
                None => None,
            },
        };

        let method = Method::from_bytes(method.as_bytes()).map_err(|_| Error::InvalidHttpMethod)?;

        let client = Client::new();

        let mut builder = client.request(method, url);

        if let Some(auth_token) = auth_token {
            builder = builder.bearer_auth(auth_token);
        }

        if let Some(json_body) = json_body {
            builder = builder.json(&json_body);
        }

        if let Some(headers) = headers {
            let headers = match headers {
                JsonValue::Object(headers) => headers,
                _ => return Err(Error::InvalidHttpHeaders),
            };

            for (key, value) in headers {
                let value = match value {
                    JsonValue::String(v) => v,
                    _ => return Err(Error::InvalidHttpHeaders),
                };
                builder = builder.header(key, value);
            }
        }

        let resp = builder.send().await?;

        let status = resp.status();
        if !status.is_success() {
            return Err(Error::HttpStatus(status.as_u16(), "".into()));
        }

        let resp_body = resp.text().await?;

        let resp_body = match serde_json::from_str(&resp_body) {
            Ok(json) => Value::Json(json),
            Err(_) => Value::String(resp_body),
        };

        let outputs = hashmap! {
            "resp_body".to_owned()=> resp_body,
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
