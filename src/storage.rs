use aws_sdk_s3::error::ProvideErrorMetadata;
use aws_sdk_s3::primitives::ByteStream;

#[derive(Debug)]
pub struct StorageError(pub String);

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "storage error: {}", self.0)
    }
}

#[derive(Clone)]
pub struct S3Storage {
    client: aws_sdk_s3::Client,
    pub bucket: String,
}

pub fn invoice_s3_key(user_id: i32, uuid: &str, extension: &str) -> String {
    format!("invoices/{user_id}/{uuid}/{uuid}.{extension}")
}

impl S3Storage {
    pub async fn from_env() -> Self {
        let bucket = std::env::var("S3_BUCKET").expect("S3_BUCKET must be set");

        let config = aws_config::load_from_env().await;

        let s3_config = if let Ok(endpoint) = std::env::var("AWS_ENDPOINT_URL") {
            aws_sdk_s3::config::Builder::from(&config)
                .endpoint_url(endpoint)
                .force_path_style(true)
                .build()
        } else {
            aws_sdk_s3::config::Builder::from(&config).build()
        };

        S3Storage {
            client: aws_sdk_s3::Client::from_conf(s3_config),
            bucket,
        }
    }

    pub async fn upload(&self, key: &str, bytes: Vec<u8>) -> Result<(), StorageError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from(bytes))
            .send()
            .await
            .map_err(|e| {
                let detail = e
                    .as_service_error()
                    .map(|se| {
                        format!(
                            "code={} message={:?}",
                            se.code().unwrap_or("?"),
                            se.message()
                        )
                    })
                    .unwrap_or_else(|| format!("{e:#}"));
                StorageError(detail)
            })?;
        Ok(())
    }

    pub async fn is_reachable(&self) -> bool {
        self.client.list_buckets().send().await.is_ok()
    }

    pub async fn download(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let output = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| StorageError(e.to_string()))?;

        let bytes = output
            .body
            .collect()
            .await
            .map_err(|e| StorageError(e.to_string()))?
            .into_bytes()
            .to_vec();

        Ok(bytes)
    }
}
