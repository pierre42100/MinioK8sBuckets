//! # Minio server controller
//!
//! Used for testing only

use crate::minio::MinioService;
use crate::temp;
use crate::utils::rand_str;
use rand::RngCore;
use std::io::ErrorKind;
use std::process::{Child, Command};
use std::time::Duration;

pub struct MinioTestServer {
    #[allow(dead_code)]
    storage_base_dir: mktemp::Temp,
    child: Child,
    pub api_port: u16,
    pub root_user: String,
    pub root_password: String,
}

impl MinioTestServer {
    pub async fn start() -> anyhow::Result<Self> {
        let storage_dir = temp::create_temp_dir()?;

        let root_user = rand_str(30);
        let root_password = rand_str(30);
        let api_port = (2000 + rand::rng().next_u64() % 5000) as u16;
        log::info!(
            "Spwan a new Minio server on port {} with root credentials {}:{}",
            api_port,
            root_user,
            root_password
        );

        let child = Command::new("minio")
            .current_dir(storage_dir.clone())
            .arg("server")
            .arg("--address")
            .arg(format!(":{api_port}"))
            .arg(storage_dir.to_str().unwrap())
            .env("MINIO_ROOT_USER", &root_user)
            .env("MINIO_ROOT_PASSWORD", &root_password)
            .spawn()?;

        let instance = Self {
            storage_base_dir: storage_dir,
            child,
            api_port,
            root_user,
            root_password,
        };

        // Wait for Minio to become ready
        std::thread::sleep(Duration::from_millis(500));
        let mut check_count = 0;
        loop {
            if check_count >= 100 {
                log::error!("Minio failed to respond properly in time!");
                return Err(std::io::Error::new(
                    ErrorKind::Other,
                    "Minio failed to respond in time!",
                )
                .into());
            }
            check_count += 1;

            std::thread::sleep(Duration::from_millis(100));

            if instance.as_service().is_ready().await {
                break;
            }
        }

        Ok(instance)
    }

    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.api_port)
    }

    /// Get a MinioService instance of this temporary server
    pub fn as_service(&self) -> MinioService {
        MinioService {
            hostname: self.base_url(),
            access_key: self.root_user.clone(),
            secret_key: self.root_password.clone(),
        }
    }
}

impl Drop for MinioTestServer {
    fn drop(&mut self) {
        if let Err(e) = self.child.kill() {
            log::error!("Failed to kill child server! {}", e);
        }
    }
}

#[cfg(test)]
mod test {
    use crate::minio_test_server::MinioTestServer;

    #[tokio::test]
    async fn start_minio() {
        let _ = env_logger::builder().is_test(true).try_init();

        let server = MinioTestServer::start().await.unwrap();
        let service = server.as_service();
        println!("{:?}", service);

        assert!(service.is_ready().await);

        // Check if minio properly exit
        drop(server);
        assert!(!service.is_ready().await);
    }
}
