use crate::{Error, Value};

use maplit::hashmap;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use reqwest::{Client, Method};

use reqwest::multipart::{Form, Part};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IpfsUpload {
    pub pinata_url: Option<String>,
    pub pinata_jwt: Option<String>,
    pub file_path: Option<String>,
}

impl IpfsUpload {
    pub(crate) async fn run(
        &self,
        mut inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, Error> {
        let pinata_url = match &self.pinata_url {
            Some(s) => s.clone(),
            None => match inputs.remove("pinata_url") {
                Some(Value::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("pinata_url".to_string())),
            },
        };

        let pinata_jwt = match &self.pinata_jwt {
            Some(s) => s.clone(),
            None => match inputs.remove("pinata_jwt") {
                Some(Value::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("pinata_jwt".to_string())),
            },
        };

        let file_path = match &self.file_path {
            Some(s) => s.clone(),
            None => match inputs.remove("file_path") {
                Some(Value::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("file_path".to_string())),
            },
        };

        let path = Path::new(&file_path).to_path_buf();

        let filename = path
            .file_name()
            .ok_or(Error::NoFilename)?
            .to_str()
            .ok_or(Error::InvalidFilename)?
            .to_owned();

        let file = tokio::fs::read(path).await?;

        let client = Client::new();

        let req = client
            .post(format!("{}/pinning/pinFileToIPFS", pinata_url))
            .bearer_auth(pinata_jwt)
            .multipart(Form::new().part("file", Part::stream(file).file_name(filename)));

        let resp = req.send().await?;

        let status = resp.status();
        if !status.is_success() {
            return Err(Error::HttpStatus(
                status.as_u16(),
                resp.text().await.unwrap_or(String::new()),
            ));
        }

        let resp_body: JsonValue = resp.json().await?;

        let image_cid = resp_body
            .as_object()
            .unwrap()
            .get("IpfsHash")
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned();

        let outputs = hashmap! {
            "image_cid".to_owned()=> Value::String(image_cid),
        };

        Ok(outputs)
    }
}
