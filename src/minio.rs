use std::process::Command;

use serde::de::DeserializeOwned;
use serde::Deserialize;

use crate::constants::{MC_EXE, SECRET_MINIO_BUCKET_ACCESS_LEN, SECRET_MINIO_BUCKET_SECRET_LEN};
use crate::crd::{BucketRetention, MinioBucketSpec, RetentionType};
use crate::utils::rand_str;

const MC_ALIAS_NAME: &str = "managedminioinst";

#[derive(thiserror::Error, Debug)]
enum MinioError {
    #[error("Failed to set 'mc' alias!")]
    SetMcAlias,
    #[error("Failed to execute 'mc' command!")]
    ExecMc,
    #[error("Failed to execute 'mc mb' command!")]
    MakeBucketFailed,
    #[error("Failed to set anonymous access!")]
    SetAnonymousAcccessFailed,
    #[error("Failed to set bucket quota!")]
    SetQuotaFailed,
    #[error("Failed to set bucket retention!")]
    SetRetentionFailed,
    #[error("Failed to set policy!")]
    ApplyPolicyFailed,
    #[error("Failed to create user!")]
    CreateUserFailed,
}

#[derive(Debug, Clone)]
pub struct MinioService {
    pub hostname: String,
    pub access_key: String,
    pub secret_key: String,
}

#[derive(Debug, Clone)]
pub struct MinioUser {
    pub username: String,
    pub password: String,
}

impl MinioUser {
    pub fn gen_random() -> Self {
        Self {
            username: rand_str(SECRET_MINIO_BUCKET_ACCESS_LEN),
            password: rand_str(SECRET_MINIO_BUCKET_SECRET_LEN),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct BucketEntry {
    pub status: String,
    key: String,
}

impl BucketEntry {
    pub fn bucket_name(&self) -> &str {
        &self.key[0..self.key.len() - 1]
    }
}

#[derive(Debug, Clone, Deserialize)]
struct BasicMinioResult {
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
struct MinioGetVersioningResult {
    pub versioning: Option<MinioVersioning>,
}

#[derive(Debug, Clone, Deserialize)]
struct MinioVersioning {
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
struct MinioAnonymousAccess {
    pub permission: String,
}

#[derive(Debug, Clone, Deserialize)]
struct MinioQuota {
    pub quota: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
struct MinioRetentionResult {
    pub enabled: Option<String>,
    pub mode: Option<String>,
    pub validity: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct MinioPolicy {
    pub policy: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Deserialize)]
struct MinioPolicyInfo {
    pub policyInfo: PolicyInfo,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Deserialize)]
struct PolicyInfo {
    Policy: serde_json::Value,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Deserialize)]
struct MinioUserListRes {
    accessKey: String,
}

#[derive(Debug, Clone, Deserialize)]
struct MinioPoliciesUserEntities {
    result: MinioPoliciesUserEntitiesInner,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Deserialize)]
struct MinioPoliciesUserEntitiesInner {
    userMappings: Option<Vec<MinioPoliciesUserEntitiesInnerUser>>,
}

#[derive(Debug, Clone, Deserialize)]
struct MinioPoliciesUserEntitiesInnerUser {
    policies: Vec<String>,
}

impl BasicMinioResult {
    pub fn success(&self) -> bool {
        self.status == "success"
    }
}

impl MinioService {
    /// Check if the Minio Service is ready to respond to our requests
    pub async fn is_ready(&self) -> bool {
        match reqwest::get(format!("{}/minio/health/live", self.hostname)).await {
            Ok(r) => {
                if r.status() == 200 {
                    log::info!("Minio is ready!");
                    return true;
                }

                log::info!(
                    "Minio not ready yet, check failed with status code {}",
                    r.status()
                );
            }
            Err(e) => log::info!("Minio not ready yet, check failed with error {e}"),
        }

        false
    }

    /// Get bucket name prefixed by mc alias name
    fn absolute_bucket_name(&self, name: &str) -> String {
        format!("{}/{name}", MC_ALIAS_NAME)
    }

    /// Execute a minio mc command
    async fn exec_mc_cmd<A>(&self, args: &[&str]) -> anyhow::Result<Vec<A>>
    where
        A: DeserializeOwned,
    {
        log::debug!("exec_mc_cmd with args {:?}", args);

        let conf_dir = mktemp::Temp::new_dir()?;
        let global_flags = ["--config-dir", conf_dir.to_str().unwrap(), "--json"];

        // First, set our alias to mc in a temporary directory
        let res = Command::new(MC_EXE)
            .args(global_flags)
            .args([
                "alias",
                "set",
                MC_ALIAS_NAME,
                self.hostname.as_str(),
                self.access_key.as_str(),
                self.secret_key.as_str(),
            ])
            .output()?;
        if res.status.code() != Some(0) {
            log::error!(
                "Failed to configure mc alias! (status code {:?}, stderr={}, stdout={})",
                res.status,
                String::from_utf8_lossy(&res.stderr),
                String::from_utf8_lossy(&res.stdout)
            );
            return Err(MinioError::SetMcAlias.into());
        }

        // Execute requested command
        let res = Command::new(MC_EXE)
            .args(global_flags)
            .args(args)
            .output()?;

        if res.status.code() != Some(0) {
            log::error!(
                "Failed execute command! (status code {:?}, stderr={}, stdout={})",
                res.status,
                String::from_utf8_lossy(&res.stderr),
                String::from_utf8_lossy(&res.stdout)
            );
            return Err(MinioError::ExecMc.into());
        }

        let stdout = String::from_utf8_lossy(&res.stdout);
        log::debug!(
            "stdout='{}' stderr='{}'",
            stdout,
            String::from_utf8_lossy(&res.stderr)
        );

        if stdout.is_empty() {
            log::info!("Command returned no result!");
            return Ok(vec![]);
        }

        let mut out = vec![];
        for l in stdout.split('\n') {
            if !l.trim().is_empty() {
                out.push(serde_json::from_str(l)?);
            }
        }
        Ok(out)
    }

    /// Get the list of buckets
    pub async fn buckets_list(&self) -> anyhow::Result<Vec<BucketEntry>> {
        self.exec_mc_cmd::<BucketEntry>(&["ls", MC_ALIAS_NAME])
            .await
    }

    /// Check if a bucket exists or not
    pub async fn bucket_exists(&self, name: &str) -> anyhow::Result<bool> {
        Ok(self
            .buckets_list()
            .await?
            .iter()
            .any(|b| b.bucket_name().eq(name)))
    }

    /// Apply bucket desired configuration. If bucket already exists, it is not dropped
    pub async fn bucket_apply(&self, b: &MinioBucketSpec) -> anyhow::Result<()> {
        // Set base parameters
        let bucket_name = format!("{}/{}", MC_ALIAS_NAME, b.name);
        let mut args = ["mb", bucket_name.as_str(), "-p"].to_vec();

        if b.lock {
            args.push("--with-lock");
        }

        let res = self.exec_mc_cmd::<BasicMinioResult>(&args).await?;
        if res.get(0).map(|r| r.success()) != Some(true) {
            return Err(MinioError::MakeBucketFailed.into());
        }

        self.bucket_set_versioning(&b.name, b.versioning || b.lock)
            .await?;
        self.bucket_set_anonymous_access(&b.name, b.anonymous_read_access)
            .await?;
        self.bucket_set_quota(&b.name, b.quota).await?;
        if b.lock {
            self.bucket_set_default_retention(&b.name, b.retention)
                .await?;
        }
        Ok(())
    }

    /// Set bucket versioning
    pub async fn bucket_set_versioning(&self, bucket: &str, enable: bool) -> anyhow::Result<()> {
        let bucket_name = self.absolute_bucket_name(bucket);

        let res = self
            .exec_mc_cmd::<BasicMinioResult>(&[
                "version",
                match enable {
                    true => "enable",
                    false => "suspend",
                },
                bucket_name.as_str(),
            ])
            .await?;

        if res.get(0).map(|r| r.success()) != Some(true) {
            return Err(MinioError::SetQuotaFailed.into());
        }
        Ok(())
    }

    /// Get current bucket versioning status
    pub async fn bucket_get_versioning(&self, bucket_name: &str) -> anyhow::Result<bool> {
        let bucket_name = self.absolute_bucket_name(bucket_name);
        Ok(self
            .exec_mc_cmd::<MinioGetVersioningResult>(&["version", "info", bucket_name.as_str()])
            .await?
            .remove(0)
            .versioning
            .map(|v| v.status.to_lowercase().eq("enabled"))
            .unwrap_or_default())
    }

    /// Set bucket anonymous access
    pub async fn bucket_set_anonymous_access(
        &self,
        bucket_name: &str,
        access: bool,
    ) -> anyhow::Result<()> {
        let target = format!("{}/*", self.absolute_bucket_name(bucket_name));

        let res = self
            .exec_mc_cmd::<BasicMinioResult>(&[
                "anonymous",
                "set",
                match access {
                    true => "download",
                    false => "private",
                },
                target.as_str(),
            ])
            .await?;

        if res.get(0).map(|r| r.success()) != Some(true) {
            return Err(MinioError::SetAnonymousAcccessFailed.into());
        }

        Ok(())
    }

    /// Get current bucket anonymous access status
    pub async fn bucket_get_anonymous_access(&self, bucket_name: &str) -> anyhow::Result<bool> {
        let bucket_name = format!("{}/*", self.absolute_bucket_name(bucket_name));
        Ok(self
            .exec_mc_cmd::<MinioAnonymousAccess>(&["anonymous", "get", bucket_name.as_str()])
            .await?
            .remove(0)
            .permission
            == "download")
    }

    /// Set bucket quota, in bytes
    pub async fn bucket_set_quota(&self, bucket: &str, quota: Option<usize>) -> anyhow::Result<()> {
        let bucket_name = self.absolute_bucket_name(bucket);

        let res = if let Some(quota) = &quota {
            let quota = format!("{}B", quota);
            self.exec_mc_cmd::<BasicMinioResult>(&[
                "quota",
                "set",
                bucket_name.as_str(),
                "--size",
                quota.as_str(),
            ])
            .await?
        } else {
            self.exec_mc_cmd::<BasicMinioResult>(&["quota", "clear", bucket_name.as_str()])
                .await?
        };

        if res.get(0).map(|r| r.success()) != Some(true) {
            return Err(MinioError::SetQuotaFailed.into());
        }
        Ok(())
    }

    /// Get current bucket quota, in bytes
    pub async fn bucket_get_quota(&self, bucket_name: &str) -> anyhow::Result<Option<usize>> {
        let bucket_name = self.absolute_bucket_name(bucket_name);
        Ok(self
            .exec_mc_cmd::<MinioQuota>(&["quota", "info", bucket_name.as_str()])
            .await?
            .remove(0)
            .quota)
    }

    /// Set bucket default retention policy
    pub async fn bucket_set_default_retention(
        &self,
        bucket_name: &str,
        retention: Option<BucketRetention>,
    ) -> anyhow::Result<()> {
        let bucket_name = self.absolute_bucket_name(bucket_name);
        let res = if let Some(retention) = &retention {
            let days = format!("{}d", retention.validity);

            self.exec_mc_cmd::<BasicMinioResult>(&[
                "retention",
                "set",
                "--default",
                match retention.r#type {
                    RetentionType::Compliance => "compliance",
                    RetentionType::Governance => "governance",
                },
                days.as_str(),
                bucket_name.as_str(),
            ])
            .await?
        } else {
            self.exec_mc_cmd::<BasicMinioResult>(&[
                "retention",
                "clear",
                "--default",
                bucket_name.as_str(),
            ])
            .await?
        };

        if res.get(0).map(|r| r.success()) != Some(true) {
            return Err(MinioError::SetRetentionFailed.into());
        }

        Ok(())
    }

    /// Get bucket default retention policy
    pub async fn bucket_get_default_retention(
        &self,
        bucket: &str,
    ) -> anyhow::Result<Option<BucketRetention>> {
        let bucket_name = self.absolute_bucket_name(bucket);
        let res = self
            .exec_mc_cmd::<MinioRetentionResult>(&[
                "retention",
                "info",
                bucket_name.as_str(),
                "--default",
            ])
            .await?
            .remove(0);

        if let (Some(mode), Some(validity), Some(enabled)) = (res.mode, res.validity, res.enabled) {
            if enabled.to_lowercase().eq("enabled") {
                return Ok(Some(BucketRetention {
                    validity: validity.to_lowercase().replace("days", "").parse()?,
                    r#type: match mode.to_lowercase().as_str() {
                        "governance" => RetentionType::Governance,
                        "compliance" => RetentionType::Compliance,
                        o => {
                            log::error!("Unknown retention type: {}", o);
                            return Ok(None);
                        }
                    },
                }));
            }
        }
        Ok(None)
    }

    /// Apply a bucket policy
    pub async fn policy_apply(&self, name: &str, content: &str) -> anyhow::Result<()> {
        let tmp_file = mktemp::Temp::new_file()?;
        std::fs::write(&tmp_file, content)?;

        let res = self
            .exec_mc_cmd::<BasicMinioResult>(&[
                "admin",
                "policy",
                "create",
                MC_ALIAS_NAME,
                name,
                tmp_file.to_str().unwrap(),
            ])
            .await?;

        if res.get(0).map(|r| r.success()) != Some(true) {
            return Err(MinioError::ApplyPolicyFailed.into());
        }

        Ok(())
    }

    /// Get the list of existing policies
    pub async fn policy_list(&self) -> anyhow::Result<Vec<String>> {
        Ok(self
            .exec_mc_cmd::<MinioPolicy>(&["admin", "policy", "list", MC_ALIAS_NAME])
            .await?
            .iter()
            .map(|p| p.policy.to_string())
            .collect())
    }

    /// Get the content of a given policy
    pub async fn policy_content(&self, name: &str) -> anyhow::Result<String> {
        let policy = self
            .exec_mc_cmd::<MinioPolicyInfo>(&["admin", "policy", "info", MC_ALIAS_NAME, name])
            .await?
            .remove(0);

        Ok(serde_json::to_string(&policy.policyInfo.Policy)?)
    }

    /// Apply a user
    pub async fn user_apply(&self, user: &MinioUser) -> anyhow::Result<()> {
        let res = self
            .exec_mc_cmd::<BasicMinioResult>(&[
                "admin",
                "user",
                "add",
                MC_ALIAS_NAME,
                user.username.as_str(),
                user.password.as_str(),
            ])
            .await?;

        if res.get(0).map(|r| r.success()) != Some(true) {
            return Err(MinioError::CreateUserFailed.into());
        }

        Ok(())
    }

    /// Get the list of users
    pub async fn user_list(&self) -> anyhow::Result<Vec<String>> {
        Ok(self
            .exec_mc_cmd::<MinioUserListRes>(&["admin", "user", "list", MC_ALIAS_NAME])
            .await?
            .iter()
            .map(|p| p.accessKey.to_string())
            .collect())
    }

    /// Attach a user to a policy
    pub async fn policy_attach_user(&self, user: &MinioUser, policy: &str) -> anyhow::Result<()> {
        // Check if the policy has already been attached to the user
        if self
            .policy_attach_get_user_list(user)
            .await?
            .contains(&policy.to_string())
        {
            return Ok(());
        }

        let res = self
            .exec_mc_cmd::<BasicMinioResult>(&[
                "admin",
                "policy",
                "attach",
                MC_ALIAS_NAME,
                policy,
                "--user",
                user.username.as_str(),
            ])
            .await?;

        if res.get(0).map(|r| r.success()) != Some(true) {
            return Err(MinioError::CreateUserFailed.into());
        }

        Ok(())
    }

    /// Get the list of entities attached to a user
    pub async fn policy_attach_get_user_list(
        &self,
        user: &MinioUser,
    ) -> anyhow::Result<Vec<String>> {
        let res = self
            .exec_mc_cmd::<MinioPoliciesUserEntities>(&[
                "admin",
                "policy",
                "entities",
                MC_ALIAS_NAME,
                "--user",
                user.username.as_str(),
            ])
            .await?
            .remove(0)
            .result
            .userMappings;

        if let Some(mapping) = res {
            if let Some(e) = mapping.get(0) {
                return Ok(e.policies.clone());
            }
        }

        Ok(vec![])
    }
}

#[cfg(test)]
mod test {
    use crate::crd::{BucketRetention, MinioBucketSpec, RetentionType};
    use crate::minio::MinioUser;
    use crate::minio_test_server::MinioTestServer;

    const TEST_BUCKET_NAME: &str = "mybucket";
    const TEST_POLICY_NAME: &str = "mypolicy";

    #[tokio::test]
    async fn list_buckets_empty_instance() {
        let srv = MinioTestServer::start().await.unwrap();
        let buckets = srv.as_service().buckets_list().await.unwrap();
        assert!(buckets.is_empty());
    }

    #[tokio::test]
    async fn bucket_exists_no_bucket() {
        let srv = MinioTestServer::start().await.unwrap();
        assert!(!srv.as_service().bucket_exists("mybucket").await.unwrap());
    }

    #[tokio::test]
    async fn bucket_basic_creation() {
        let _ = env_logger::builder().is_test(true).try_init();

        let srv = MinioTestServer::start().await.unwrap();
        let service = srv.as_service();
        service
            .bucket_apply(&MinioBucketSpec {
                instance: "".to_string(),
                name: TEST_BUCKET_NAME.to_string(),
                secret: "".to_string(),
                anonymous_read_access: false,
                versioning: false,
                quota: None,
                lock: false,
                retention: None,
            })
            .await
            .unwrap();
        assert!(service.bucket_exists(TEST_BUCKET_NAME).await.unwrap());
    }

    #[tokio::test]
    async fn bucket_creation_with_anonymous_access() {
        let _ = env_logger::builder().is_test(true).try_init();

        let srv = MinioTestServer::start().await.unwrap();
        let service = srv.as_service();
        service
            .bucket_apply(&MinioBucketSpec {
                instance: "".to_string(),
                name: TEST_BUCKET_NAME.to_string(),
                secret: "".to_string(),
                anonymous_read_access: true,
                versioning: false,
                quota: None,
                lock: false,
                retention: None,
            })
            .await
            .unwrap();

        assert!(service.bucket_exists(TEST_BUCKET_NAME).await.unwrap());
        assert!(service
            .bucket_get_anonymous_access(TEST_BUCKET_NAME)
            .await
            .unwrap());
        assert_eq!(
            reqwest::get(format!("{}/{}/test", service.hostname, TEST_BUCKET_NAME))
                .await
                .unwrap()
                .status()
                .as_u16(),
            404
        );
    }

    #[tokio::test]
    async fn bucket_creation_without_anonymous_access() {
        let _ = env_logger::builder().is_test(true).try_init();

        let srv = MinioTestServer::start().await.unwrap();
        let service = srv.as_service();
        service
            .bucket_apply(&MinioBucketSpec {
                instance: "".to_string(),
                name: TEST_BUCKET_NAME.to_string(),
                secret: "".to_string(),
                anonymous_read_access: false,
                versioning: false,
                quota: None,
                lock: false,
                retention: None,
            })
            .await
            .unwrap();

        assert!(service.bucket_exists(TEST_BUCKET_NAME).await.unwrap());
        assert_eq!(
            reqwest::get(format!("{}/{}/test", service.hostname, TEST_BUCKET_NAME))
                .await
                .unwrap()
                .status()
                .as_u16(),
            403
        );
    }

    #[tokio::test]
    async fn bucket_creation_without_anonymous_access_updating_status() {
        let _ = env_logger::builder().is_test(true).try_init();

        let srv = MinioTestServer::start().await.unwrap();
        let service = srv.as_service();
        service
            .bucket_apply(&MinioBucketSpec {
                instance: "".to_string(),
                name: TEST_BUCKET_NAME.to_string(),
                secret: "".to_string(),
                anonymous_read_access: false,
                versioning: false,
                quota: None,
                lock: false,
                retention: None,
            })
            .await
            .unwrap();

        assert!(service.bucket_exists(TEST_BUCKET_NAME).await.unwrap());
        let test_url = format!("{}/{}/test", service.hostname, TEST_BUCKET_NAME);
        assert_eq!(
            reqwest::get(&test_url).await.unwrap().status().as_u16(),
            403
        );
        service
            .bucket_set_anonymous_access(TEST_BUCKET_NAME, true)
            .await
            .unwrap();
        assert!(service
            .bucket_get_anonymous_access(TEST_BUCKET_NAME)
            .await
            .unwrap());
        assert_eq!(
            reqwest::get(&test_url).await.unwrap().status().as_u16(),
            404
        );

        service
            .bucket_set_anonymous_access(TEST_BUCKET_NAME, false)
            .await
            .unwrap();
        assert!(!service
            .bucket_get_anonymous_access(TEST_BUCKET_NAME)
            .await
            .unwrap());
        assert_eq!(
            reqwest::get(&test_url).await.unwrap().status().as_u16(),
            403
        );
    }

    #[tokio::test]
    async fn bucket_with_versioning() {
        let _ = env_logger::builder().is_test(true).try_init();

        let srv = MinioTestServer::start().await.unwrap();
        let service = srv.as_service();
        service
            .bucket_apply(&MinioBucketSpec {
                instance: "".to_string(),
                name: TEST_BUCKET_NAME.to_string(),
                secret: "".to_string(),
                anonymous_read_access: false,
                versioning: true,
                quota: None,
                lock: false,
                retention: None,
            })
            .await
            .unwrap();

        assert!(service.bucket_exists(TEST_BUCKET_NAME).await.unwrap());
        assert!(service
            .bucket_get_versioning(TEST_BUCKET_NAME)
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn bucket_without_versioning() {
        let _ = env_logger::builder().is_test(true).try_init();

        let srv = MinioTestServer::start().await.unwrap();
        let service = srv.as_service();
        service
            .bucket_apply(&MinioBucketSpec {
                instance: "".to_string(),
                name: TEST_BUCKET_NAME.to_string(),
                secret: "".to_string(),
                anonymous_read_access: false,
                versioning: false,
                quota: None,
                lock: false,
                retention: None,
            })
            .await
            .unwrap();

        assert!(service.bucket_exists(TEST_BUCKET_NAME).await.unwrap());
        assert!(!service
            .bucket_get_versioning(TEST_BUCKET_NAME)
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn change_versioning() {
        let _ = env_logger::builder().is_test(true).try_init();

        let srv = MinioTestServer::start().await.unwrap();
        let service = srv.as_service();
        service
            .bucket_apply(&MinioBucketSpec {
                instance: "".to_string(),
                name: TEST_BUCKET_NAME.to_string(),
                secret: "".to_string(),
                anonymous_read_access: false,
                versioning: false,
                quota: None,
                lock: false,
                retention: None,
            })
            .await
            .unwrap();

        assert!(service.bucket_exists(TEST_BUCKET_NAME).await.unwrap());
        assert!(!service
            .bucket_get_versioning(TEST_BUCKET_NAME)
            .await
            .unwrap());
        service
            .bucket_set_versioning(TEST_BUCKET_NAME, true)
            .await
            .unwrap();
        assert!(service
            .bucket_get_versioning(TEST_BUCKET_NAME)
            .await
            .unwrap());
        service
            .bucket_set_versioning(TEST_BUCKET_NAME, false)
            .await
            .unwrap();
        assert!(!service
            .bucket_get_versioning(TEST_BUCKET_NAME)
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn bucket_without_quota() {
        let _ = env_logger::builder().is_test(true).try_init();

        let srv = MinioTestServer::start().await.unwrap();
        let service = srv.as_service();
        service
            .bucket_apply(&MinioBucketSpec {
                instance: "".to_string(),
                name: TEST_BUCKET_NAME.to_string(),
                secret: "".to_string(),
                anonymous_read_access: false,
                versioning: false,
                quota: None,
                lock: false,
                retention: None,
            })
            .await
            .unwrap();

        assert!(service.bucket_exists(TEST_BUCKET_NAME).await.unwrap());
        assert_eq!(
            service.bucket_get_quota(TEST_BUCKET_NAME).await.unwrap(),
            None
        );

        service
            .bucket_set_quota(TEST_BUCKET_NAME, Some(5122600))
            .await
            .unwrap();
        assert_eq!(
            service.bucket_get_quota(TEST_BUCKET_NAME).await.unwrap(),
            Some(5122600)
        );

        service
            .bucket_set_quota(TEST_BUCKET_NAME, None)
            .await
            .unwrap();
        assert_eq!(
            service.bucket_get_quota(TEST_BUCKET_NAME).await.unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn bucket_with_quota() {
        let _ = env_logger::builder().is_test(true).try_init();

        let srv = MinioTestServer::start().await.unwrap();
        let service = srv.as_service();
        service
            .bucket_apply(&MinioBucketSpec {
                instance: "".to_string(),
                name: TEST_BUCKET_NAME.to_string(),
                secret: "".to_string(),
                anonymous_read_access: false,
                versioning: false,
                quota: Some(42300),
                lock: false,
                retention: None,
            })
            .await
            .unwrap();

        assert!(service.bucket_exists(TEST_BUCKET_NAME).await.unwrap());
        assert_eq!(
            service.bucket_get_quota(TEST_BUCKET_NAME).await.unwrap(),
            Some(42300)
        );
    }

    #[tokio::test]
    async fn bucket_with_retention() {
        let _ = env_logger::builder().is_test(true).try_init();

        let srv = MinioTestServer::start().await.unwrap();
        let service = srv.as_service();
        service
            .bucket_apply(&MinioBucketSpec {
                instance: "".to_string(),
                name: TEST_BUCKET_NAME.to_string(),
                secret: "".to_string(),
                anonymous_read_access: false,
                versioning: false,
                quota: Some(42300),
                lock: true,
                retention: Some(BucketRetention {
                    validity: 10,
                    r#type: RetentionType::Governance,
                }),
            })
            .await
            .unwrap();

        assert!(service.bucket_exists(TEST_BUCKET_NAME).await.unwrap());
        assert_eq!(
            service
                .bucket_get_default_retention(TEST_BUCKET_NAME)
                .await
                .unwrap(),
            Some(BucketRetention {
                validity: 10,
                r#type: RetentionType::Governance
            })
        );

        service
            .bucket_set_default_retention(TEST_BUCKET_NAME, None)
            .await
            .unwrap();
        assert_eq!(
            service
                .bucket_get_default_retention(TEST_BUCKET_NAME)
                .await
                .unwrap(),
            None
        );

        service
            .bucket_set_default_retention(
                TEST_BUCKET_NAME,
                Some(BucketRetention {
                    validity: 42,
                    r#type: RetentionType::Compliance,
                }),
            )
            .await
            .unwrap();
        assert_eq!(
            service
                .bucket_get_default_retention(TEST_BUCKET_NAME)
                .await
                .unwrap(),
            Some(BucketRetention {
                validity: 42,
                r#type: RetentionType::Compliance
            })
        );

        service
            .bucket_set_default_retention(
                TEST_BUCKET_NAME,
                Some(BucketRetention {
                    validity: 21,
                    r#type: RetentionType::Governance,
                }),
            )
            .await
            .unwrap();
        assert_eq!(
            service
                .bucket_get_default_retention(TEST_BUCKET_NAME)
                .await
                .unwrap(),
            Some(BucketRetention {
                validity: 21,
                r#type: RetentionType::Governance
            })
        );
    }

    #[tokio::test]
    async fn bucket_without_retention() {
        let _ = env_logger::builder().is_test(true).try_init();

        let srv = MinioTestServer::start().await.unwrap();
        let service = srv.as_service();
        service
            .bucket_apply(&MinioBucketSpec {
                instance: "".to_string(),
                name: TEST_BUCKET_NAME.to_string(),
                secret: "".to_string(),
                anonymous_read_access: false,
                versioning: false,
                quota: Some(42300),
                lock: true,
                retention: None,
            })
            .await
            .unwrap();

        assert!(service.bucket_exists(TEST_BUCKET_NAME).await.unwrap());
        assert_eq!(
            service
                .bucket_get_default_retention(TEST_BUCKET_NAME)
                .await
                .unwrap(),
            None
        );
    }

    fn unify_policy(p: &str) -> String {
        serde_json::to_string(&serde_json::from_str::<serde_json::Value>(p).unwrap()).unwrap()
    }

    #[tokio::test]
    async fn policy_apply() {
        let _ = env_logger::builder().is_test(true).try_init();

        let srv = MinioTestServer::start().await.unwrap();
        let service = srv.as_service();

        let policy_1 = unify_policy(include_str!("../test/test-policy1.json"));
        let policy_2 = unify_policy(include_str!("../test/test-policy2.json"));

        assert_ne!(policy_1, policy_2);

        assert!(!service
            .policy_list()
            .await
            .unwrap()
            .contains(&TEST_POLICY_NAME.to_string()));

        service
            .policy_apply(TEST_POLICY_NAME, &policy_1)
            .await
            .unwrap();
        assert!(service
            .policy_list()
            .await
            .unwrap()
            .contains(&TEST_POLICY_NAME.to_string()));
        assert_eq!(
            unify_policy(&service.policy_content(TEST_POLICY_NAME).await.unwrap()),
            policy_1
        );

        service
            .policy_apply(TEST_POLICY_NAME, &policy_2)
            .await
            .unwrap();
        assert!(service
            .policy_list()
            .await
            .unwrap()
            .contains(&TEST_POLICY_NAME.to_string()));
        assert_eq!(
            unify_policy(&service.policy_content(TEST_POLICY_NAME).await.unwrap()),
            policy_2
        );
    }

    #[tokio::test]
    async fn policy_user() {
        let _ = env_logger::builder().is_test(true).try_init();

        let srv = MinioTestServer::start().await.unwrap();
        let service = srv.as_service();

        let user = MinioUser::gen_random();

        assert!(!service.user_list().await.unwrap().contains(&user.username));
        service.user_apply(&user).await.unwrap();
        assert!(service.user_list().await.unwrap().contains(&user.username));
    }

    #[tokio::test]
    async fn attach_policy_user() {
        let _ = env_logger::builder().is_test(true).try_init();

        let srv = MinioTestServer::start().await.unwrap();
        let service = srv.as_service();

        let user = MinioUser::gen_random();

        service.user_apply(&user).await.unwrap();
        service
            .policy_apply(TEST_POLICY_NAME, include_str!("../test/test-policy1.json"))
            .await
            .unwrap();

        assert!(!service
            .policy_attach_get_user_list(&user)
            .await
            .unwrap()
            .contains(&TEST_POLICY_NAME.to_string()));
        service
            .policy_attach_user(&user, TEST_POLICY_NAME)
            .await
            .unwrap();
        assert!(service
            .policy_attach_get_user_list(&user)
            .await
            .unwrap()
            .contains(&TEST_POLICY_NAME.to_string()));
    }
}
