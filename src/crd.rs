use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[kube(
    group = "communiquons.org",
    version = "v1",
    kind = "MinioInstance",
    namespaced
)]
pub struct MinioInstanceSpec {
    pub endpoint: String,
    pub credentials: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Copy, Clone, JsonSchema, PartialEq, Eq)]
pub enum RetentionType {
    #[default]
    #[serde(rename_all = "lowercase")]
    Compliance,
    #[serde(rename_all = "lowercase")]
    Governance,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Copy, JsonSchema, PartialEq, Eq)]
pub struct BucketRetention {
    pub validity: usize,
    pub r#type: RetentionType,
}

#[derive(CustomResource, Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[kube(
    group = "communiquons.org",
    version = "v1",
    kind = "MinioBucket",
    namespaced
)]
pub struct MinioBucketSpec {
    pub instance: String,
    pub name: String,
    pub secret: String,
    #[serde(default)]
    pub anonymous_read_access: bool,
    #[serde(default)]
    pub versioning: bool,
    pub quota: Option<usize>,
    #[serde(default)]
    pub lock: bool,
    pub retention: Option<BucketRetention>,
}
