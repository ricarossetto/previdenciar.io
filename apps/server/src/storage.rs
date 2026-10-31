use aws_sdk_s3::presigning::PresigningConfig;
use std::path::Path;
use std::time::Duration;

#[derive(Clone)]
pub struct S3Store {
    client: aws_sdk_s3::Client,
    bucket: String,
    region: String,
}

impl S3Store {
    pub async fn new(bucket: String, region: String) -> Option<Self> {
        if bucket.trim().is_empty() {
            return None;
        }
        let shared = aws_config::load_from_env().await;
        let client = aws_sdk_s3::Client::new(&shared);
        Some(Self { client, bucket, region })
    }

    pub async fn put_bytes(&self, key: &str, bytes: Vec<u8>, content_type: &str) -> Result<(), String> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .body(bytes.into())
            .send()
            .await
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<(), String> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub async fn presigned_get(&self, key: &str, seconds: u64) -> Result<String, String> {
        let config = PresigningConfig::expires_in(Duration::from_secs(seconds.clamp(60, 3600)))
            .map_err(|error| error.to_string())?;
        let request = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(config)
            .await
            .map_err(|error| error.to_string())?;
        Ok(request.uri().to_string())
    }

    pub async fn download_knowledge(&self, key: &str, destination: &Path) -> Result<(), String> {
        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|error| error.to_string())?;
        let body = response
            .body
            .collect()
            .await
            .map_err(|error| error.to_string())?
            .into_bytes();
        if body.len() > 512 * 1024 * 1024 {
            return Err("Pacote de conhecimento excede o limite de 512 MiB.".into());
        }
        std::fs::create_dir_all(destination).map_err(|error| error.to_string())?;
        let decoder = flate2::read::GzDecoder::new(body.as_ref());
        let mut archive = tar::Archive::new(decoder);
        archive.unpack(destination).map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn region(&self) -> &str {
        &self.region
    }
}
