use futures::TryStreamExt;
use k8s_openapi::api::core::v1::Secret;
use kube::runtime::{watcher, WatchStreamExt};
use kube::{Api, Client};
use minio_operator::constants::{
    SECRET_MINIO_BUCKET_ACCESS_KEY, SECRET_MINIO_BUCKET_SECRET_KEY,
    SECRET_MINIO_INSTANCE_ACCESS_KEY, SECRET_MINIO_INSTANCE_SECRET_KEY,
};
use minio_operator::crd::{MinioBucket, MinioInstance};
use minio_operator::minio::{MinioService, MinioUser};
use minio_operator::secrets::{create_secret, read_secret_str};
use std::collections::BTreeMap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let client = Client::try_default().await?;

    let buckets: Api<MinioBucket> = Api::default_namespaced(client.clone());

    // Listen for events / buckets creation or update (deletion is not supported)
    let wc = watcher::Config::default();
    let bw = watcher(buckets, wc).applied_objects();
    futures::pin_mut!(bw);

    while let Some(b) = bw.try_next().await? {
        if let Err(e) = apply_bucket(&b, &client).await {
            log::error!(
                "Failed to apply desired configuration for applied bucket {} : {}",
                b.spec.name,
                e
            )
        }
    }

    Ok(())
}

/// Make sure a bucket is compliant with a desired configuration
async fn apply_bucket(b: &MinioBucket, client: &Client) -> anyhow::Result<()> {
    log::info!("Apply configuration for bucket {}", b.spec.name);

    // Get instance information
    let instances: Api<MinioInstance> = Api::default_namespaced(client.clone());
    let instance = instances.get(&b.spec.instance).await?;

    // Get instance configuration
    let secrets: Api<Secret> = Api::default_namespaced(client.clone());
    let instance_secret = secrets.get(&instance.spec.credentials).await?;
    let service = MinioService {
        hostname: instance.spec.endpoint,
        access_key: read_secret_str(&instance_secret, SECRET_MINIO_INSTANCE_ACCESS_KEY)?,
        secret_key: read_secret_str(&instance_secret, SECRET_MINIO_INSTANCE_SECRET_KEY)?,
    };

    // Get user key & password
    let user_secret = match secrets.get_opt(&b.spec.secret).await? {
        Some(s) => s,
        None => {
            log::info!(
                "Needs to create the secret {} for the bucket {}",
                b.spec.secret,
                b.spec.name
            );

            // The secret needs to be created
            let new_user = MinioUser::gen_random();
            create_secret(
                &secrets,
                &b.spec.secret,
                BTreeMap::from([
                    (
                        SECRET_MINIO_BUCKET_ACCESS_KEY.to_string(),
                        new_user.username,
                    ),
                    (
                        SECRET_MINIO_BUCKET_SECRET_KEY.to_string(),
                        new_user.password,
                    ),
                ]),
            )
            .await?
        }
    };
    let user = MinioUser {
        username: read_secret_str(&user_secret, SECRET_MINIO_BUCKET_ACCESS_KEY)?,
        password: read_secret_str(&user_secret, SECRET_MINIO_BUCKET_SECRET_KEY)?,
    };

    log::debug!("Create or update bucket...");
    service.bucket_apply(&b.spec).await?;

    let policy_name = format!("bucket-{}", b.spec.name);
    log::debug!("Create or update policy '{policy_name}'...");
    let policy_content =
        include_str!("policy_template.json").replace("{{ bucket }}", b.spec.name.as_str());
    service.policy_apply(&policy_name, &policy_content).await?;

    log::debug!("Create or update user '{}'...", user.username);
    service.user_apply(&user).await?;

    log::debug!("Attach policy '{policy_name}' to user...");
    service.policy_attach_user(&user, &policy_name).await?;

    log::debug!("Successfully applied desired configuration!");

    Ok(())
}
