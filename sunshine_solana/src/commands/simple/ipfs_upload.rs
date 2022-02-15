use crate::{Error, Value};

use maplit::hashmap;
use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use reqwest::{Client, Method};

use reqwest::multipart::{Form, Part};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IpfsUpload {
    pub pinata_url: Option<String>,
    pub pinata_key: Option<String>,
    pub pinata_secret: Option<String>,
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

        let pinata_key = match &self.pinata_key {
            Some(s) => s.clone(),
            None => match inputs.remove("pinata_key") {
                Some(Value::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("pinata_key".to_string())),
            },
        };

        let pinata_secret = match &self.pinata_secret {
            Some(s) => s.clone(),
            None => match inputs.remove("pinata_secret") {
                Some(Value::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("pinata_secret".to_string())),
            },
        };

        let file_path = match &self.file_path {
            Some(s) => s.clone(),
            None => match inputs.remove("file_path") {
                Some(Value::String(s)) => s,
                _ => return Err(Error::ArgumentNotFound("file_path".to_string())),
            },
        };

        let path = Path::new(file_path);

        let filename = path.filename().ok_or()?;

        let client = Client::new();

        let mut builder = client
            .post(format!("{}/pinning/pinFileToIPFS"))
            //.header()
            //.header()
            .multipart(Form::new().part(
                "file",
                Part::stream(tokio::fs::read(path)).filename(filename),
            ));

        let resp = builder.send().await?;

        let status = resp.status();
        if !status.is_success() {
            return Err(Error::HttpStatus(status.as_u16()));
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
