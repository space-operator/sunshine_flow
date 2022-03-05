use crate::{Error, NftMetadata, Value};

use maplit::hashmap;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use reqwest::{Client, Method};

use reqwest::multipart::{Form, Part};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IpfsNftUpload {
    pub pinata_url: Option<String>,
    pub pinata_jwt: Option<String>,
    pub metadata: Option<NftMetadata>,
}

impl IpfsNftUpload {
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

        let mut metadata = match &self.metadata {
            Some(s) => s.clone(),
            None => match inputs.remove("metadata") {
                Some(Value::NftMetadata(s)) => s,
                _ => return Err(Error::ArgumentNotFound("metadata".to_string())),
            },
        };

        metadata.image = format!(
            "ipfs://{}",
            upload_file(&pinata_url, &pinata_jwt, &metadata.image).await?
        );

        for file in metadata.properties.files.iter_mut() {
            file.uri = format!(
                "ipfs://{}",
                upload_file(&pinata_url, &pinata_jwt, &file.uri).await?
            );
        }

        let metadata_cid = upload_metadata(&pinata_url, &pinata_jwt, &metadata).await?;

        let outputs = hashmap! {
            "metadata_cid".to_owned()=> Value::String(metadata_cid.clone()),
            "metadata".to_owned() => Value::NftMetadata(metadata),
            "metadata_url".to_owned() => Value::String(format!("ipfs://{}", metadata_cid)),
        };

        Ok(outputs)
    }
}

async fn upload_file(pinata_url: &str, pinata_jwt: &str, file_path: &str) -> Result<String, Error> {
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

    Ok(resp_body
        .as_object()
        .unwrap()
        .get("IpfsHash")
        .unwrap()
        .as_str()
        .unwrap()
        .to_owned())
}

async fn upload_metadata(
    pinata_url: &str,
    pinata_jwt: &str,
    metadata: &NftMetadata,
) -> Result<String, Error> {
    let client = Client::new();

    let bytes = serde_json::to_vec(&metadata).unwrap();

    let req = client
        .post(format!("{}/pinning/pinFileToIPFS", pinata_url))
        .bearer_auth(pinata_jwt)
        .multipart(Form::new().part("file", Part::bytes(bytes).file_name("metadata.json")));

    let resp = req.send().await?;

    let status = resp.status();
    if !status.is_success() {
        return Err(Error::HttpStatus(
            status.as_u16(),
            resp.text().await.unwrap_or(String::new()),
        ));
    }

    let resp_body: JsonValue = resp.json().await?;

    Ok(resp_body
        .as_object()
        .unwrap()
        .get("IpfsHash")
        .unwrap()
        .as_str()
        .unwrap()
        .to_owned())
}
