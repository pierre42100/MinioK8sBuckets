use k8s_openapi::api::core::v1::Secret;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::api::PostParams;
use kube::Api;
use std::collections::BTreeMap;

#[derive(thiserror::Error, Debug)]
enum SecretError {
    #[error("Secret has no data!")]
    MissingData,
    #[error("The key '{0}' is not present in the secret!")]
    MissingKey(String),
}

/// Attempt to read a value contained in a secret. Returns an error in case
/// of failure
pub fn read_secret_str(s: &Secret, key: &str) -> anyhow::Result<String> {
    let data = s.data.as_ref().ok_or(SecretError::MissingData)?;

    let value = data
        .get(key)
        .ok_or(SecretError::MissingKey(key.to_string()))?;

    Ok(String::from_utf8(value.0.clone())?)
}

/// Create a secret consisting only of string key / value pairs
pub async fn create_secret(
    secrets: &Api<Secret>,
    name: &str,
    values: BTreeMap<String, String>,
) -> anyhow::Result<Secret> {
    Ok(secrets
        .create(
            &PostParams::default(),
            &Secret {
                data: None,
                immutable: None,
                metadata: ObjectMeta {
                    annotations: None,
                    creation_timestamp: None,
                    deletion_grace_period_seconds: None,
                    deletion_timestamp: None,
                    finalizers: None,
                    generate_name: None,
                    generation: None,
                    labels: Some(BTreeMap::from([(
                        "created-by".to_string(),
                        "miniok8sbuckets".to_string(),
                    )])),
                    managed_fields: None,
                    name: Some(name.to_string()),
                    namespace: None,
                    owner_references: None,
                    resource_version: None,
                    self_link: None,
                    uid: None,
                },
                string_data: Some(values),
                type_: None,
            },
        )
        .await?)
}
