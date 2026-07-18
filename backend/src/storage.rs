use std::{error::Error, io, sync::Arc, time::Duration};

use aws_credential_types::Credentials;
use aws_sdk_s3::{
    config::{Builder as S3ConfigBuilder, Region, SharedCredentialsProvider},
    presigning::PresigningConfig,
    primitives::ByteStream,
    Client,
};

use crate::config::ObjectStorageConfig;

pub type StorageResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Clone)]
pub struct ObjectStorage {
    client: Client,
    bucket: Arc<str>,
    prefix: Arc<str>,
}

pub struct StoredObject {
    pub bytes: Vec<u8>,
    pub content_type: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageContentType {
    pub content_type: &'static str,
    pub extension: &'static str,
}

impl ObjectStorage {
    pub fn from_config(config: &ObjectStorageConfig) -> Self {
        let credentials = SharedCredentialsProvider::new(Credentials::new(
            config.access_key_id.clone(),
            config.secret_access_key.clone(),
            None,
            None,
            "object-storage-config",
        ));
        let mut s3_config = S3ConfigBuilder::new()
            .region(Region::new(config.region.clone()))
            .credentials_provider(credentials)
            .force_path_style(true);
        if !config.endpoint.trim().is_empty() {
            s3_config = s3_config.endpoint_url(config.endpoint.clone());
        }
        let s3_config = s3_config.build();

        Self {
            client: Client::from_conf(s3_config),
            bucket: Arc::from(config.bucket.as_str()),
            prefix: Arc::from(normalize_prefix(&config.prefix)),
        }
    }

    pub fn bucket(&self) -> &str {
        &self.bucket
    }

    pub fn is_configured(&self) -> bool {
        !self.bucket.is_empty()
    }

    fn ensure_configured(&self) -> StorageResult<()> {
        if self.is_configured() {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "object storage is not configured for this deployment",
            )
            .into())
        }
    }

    pub fn object_key(&self, key: &str) -> String {
        let key = key.trim_start_matches('/');
        if self.prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}/{key}", self.prefix)
        }
    }

    pub async fn put_object(
        &self,
        key: &str,
        bytes: Vec<u8>,
        content_type: Option<&str>,
    ) -> StorageResult<String> {
        self.ensure_configured()?;
        let object_key = self.object_key(key);
        let mut request = self
            .client
            .put_object()
            .bucket(self.bucket.as_ref())
            .key(&object_key)
            .body(ByteStream::from(bytes));

        if let Some(content_type) = content_type {
            request = request.content_type(content_type);
        }

        request.send().await?;
        Ok(object_key)
    }

    pub async fn delete_object_key(&self, object_key: &str) -> StorageResult<()> {
        self.ensure_configured()?;
        self.client
            .delete_object()
            .bucket(self.bucket.as_ref())
            .key(object_key)
            .send()
            .await?;

        Ok(())
    }

    pub async fn get_object(&self, key: &str) -> StorageResult<StoredObject> {
        self.ensure_configured()?;
        let response = self
            .client
            .get_object()
            .bucket(self.bucket.as_ref())
            .key(self.object_key(key))
            .send()
            .await?;
        let content_type = response.content_type().map(str::to_string);
        let bytes = response.body.collect().await?.into_bytes().to_vec();

        Ok(StoredObject {
            bytes,
            content_type,
        })
    }

    pub async fn presigned_get_url(
        &self,
        key: &str,
        expires_in: Duration,
    ) -> StorageResult<String> {
        self.ensure_configured()?;
        let presigning_config = PresigningConfig::expires_in(expires_in)?;
        let presigned = self
            .client
            .get_object()
            .bucket(self.bucket.as_ref())
            .key(self.object_key(key))
            .presigned(presigning_config)
            .await?;

        Ok(presigned.uri().to_string())
    }

    pub async fn presigned_get_url_for_object_key(
        &self,
        object_key: &str,
        expires_in: Duration,
    ) -> StorageResult<String> {
        self.ensure_configured()?;
        let presigning_config = PresigningConfig::expires_in(expires_in)?;
        let presigned = self
            .client
            .get_object()
            .bucket(self.bucket.as_ref())
            .key(object_key)
            .presigned(presigning_config)
            .await?;

        Ok(presigned.uri().to_string())
    }
}

fn normalize_prefix(prefix: &str) -> String {
    prefix.trim_matches('/').to_string()
}

pub fn standard_image_type(content_type: &str) -> Option<ImageContentType> {
    match content_type {
        "image/jpeg" | "image/jpg" => Some(ImageContentType {
            content_type: "image/jpeg",
            extension: "jpg",
        }),
        "image/png" => Some(ImageContentType {
            content_type: "image/png",
            extension: "png",
        }),
        "image/webp" => Some(ImageContentType {
            content_type: "image/webp",
            extension: "webp",
        }),
        "image/gif" => Some(ImageContentType {
            content_type: "image/gif",
            extension: "gif",
        }),
        _ => None,
    }
}

pub fn safe_key_part(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{safe_key_part, standard_image_type, ImageContentType};

    #[test]
    fn standard_image_type_accepts_common_web_images() {
        assert_eq!(
            standard_image_type("image/jpeg"),
            Some(ImageContentType {
                content_type: "image/jpeg",
                extension: "jpg",
            })
        );
        assert_eq!(
            standard_image_type("image/png"),
            Some(ImageContentType {
                content_type: "image/png",
                extension: "png",
            })
        );
        assert_eq!(
            standard_image_type("image/webp"),
            Some(ImageContentType {
                content_type: "image/webp",
                extension: "webp",
            })
        );
        assert_eq!(
            standard_image_type("image/gif"),
            Some(ImageContentType {
                content_type: "image/gif",
                extension: "gif",
            })
        );
    }

    #[test]
    fn safe_key_part_replaces_path_separators() {
        assert_eq!(safe_key_part("user|abc/123"), "user_abc_123");
    }
}
