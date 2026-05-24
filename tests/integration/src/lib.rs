//! Test harness that launches the counter sample silo and bridge with dynamic
//! ports and exposes a connected [`OrleansClient`] to integration tests.
//!
//! The harness shells out to the .NET SDK. Tests that use it are marked
//! `#[ignore]` so `cargo test --workspace` stays green in environments without
//! a .NET toolchain; CI runs them explicitly with `-- --ignored`.

use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use orleans_rust_client::OrleansClient;

/// A running silo + bridge pair. Dropping it terminates both processes.
pub struct TestCluster {
    silo: Child,
    bridge: Child,
    /// gRPC endpoint of the bridge, e.g. `http://127.0.0.1:50123`.
    pub bridge_url: String,
    /// Orleans service id the cluster was started with.
    pub service_id: String,
    /// Orleans cluster id the cluster was started with.
    pub cluster_id: String,
}

impl TestCluster {
    /// Build the sample, start the silo, wait for it to listen, start the
    /// bridge, and wait for it to report healthy.
    ///
    /// # Errors
    /// Returns an error if the toolchain is missing, a build fails, or either
    /// process does not become ready within the timeout.
    pub async fn start() -> anyhow::Result<Self> {
        let root = repo_root()?;
        let dotnet = std::env::var("DOTNET").unwrap_or_else(|_| "dotnet".to_owned());
        let solution_dir = root.join("examples/counter/dotnet");

        // Build the whole solution once up front so the two `dotnet run` calls
        // below do not race each other building shared project outputs.
        build(&dotnet, &root.join("orleans-rust-client.slnx"))?;

        let gateway_port = free_port()?;
        let silo_port = free_port()?;
        let bridge_port = free_port()?;
        let bridge_url = format!("http://127.0.0.1:{bridge_port}");
        let service_id = "counter-sample".to_owned();
        let cluster_id = "dev".to_owned();

        // Run the built assemblies directly rather than via `dotnet run`: the
        // `run` wrapper spawns the app as a child, so killing the wrapper would
        // orphan it. Children use null stdio so they neither spam the test
        // output nor hold the test's stdout pipe open after it exits.
        let silo = Command::new(&dotnet)
            .arg(dll(&solution_dir, "Counter.Silo"))
            .env("ORLEANS_GATEWAY_PORT", gateway_port.to_string())
            .env("ORLEANS_SILO_PORT", silo_port.to_string())
            .env("ORLEANS_SERVICE_ID", &service_id)
            .env("ORLEANS_CLUSTER_ID", &cluster_id)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        wait_for_port(gateway_port, Duration::from_secs(90))?;

        let bridge = Command::new(&dotnet)
            .arg(dll(&solution_dir, "Counter.Bridge"))
            .env("ASPNETCORE_URLS", &bridge_url)
            .env("ORLEANS_GATEWAY_PORT", gateway_port.to_string())
            .env("ORLEANS_SERVICE_ID", &service_id)
            .env("ORLEANS_CLUSTER_ID", &cluster_id)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        let cluster = TestCluster {
            silo,
            bridge,
            bridge_url,
            service_id,
            cluster_id,
        };

        cluster.wait_for_health(Duration::from_secs(90)).await?;
        Ok(cluster)
    }

    /// Connect a fresh client to the bridge.
    ///
    /// # Errors
    /// Propagates connection failures.
    pub async fn client(&self) -> anyhow::Result<OrleansClient> {
        Ok(OrleansClient::connect(&self.bridge_url).await?)
    }

    async fn wait_for_health(&self, timeout: Duration) -> anyhow::Result<()> {
        let deadline = Instant::now() + timeout;
        let mut last_err: Option<String> = None;
        while Instant::now() < deadline {
            match OrleansClient::connect(&self.bridge_url).await {
                Ok(client) => match client.health().await {
                    Ok(health) if health.status.eq_ignore_ascii_case("healthy") => return Ok(()),
                    Ok(health) => last_err = Some(format!("status={}", health.status)),
                    Err(e) => last_err = Some(e.to_string()),
                },
                Err(e) => last_err = Some(e.to_string()),
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        anyhow::bail!(
            "bridge did not become healthy within {timeout:?}: {}",
            last_err.unwrap_or_else(|| "no response".to_owned())
        )
    }
}

impl Drop for TestCluster {
    fn drop(&mut self) {
        let _ = self.bridge.kill();
        let _ = self.silo.kill();
        let _ = self.bridge.wait();
        let _ = self.silo.wait();
    }
}

fn dll(solution_dir: &std::path::Path, project: &str) -> PathBuf {
    solution_dir
        .join(project)
        .join("bin/Release/net10.0")
        .join(format!("{project}.dll"))
}

fn repo_root() -> anyhow::Result<PathBuf> {
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

fn free_port() -> anyhow::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

fn wait_for_port(port: u16, timeout: Duration) -> anyhow::Result<()> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    anyhow::bail!("port {port} did not open within {timeout:?}")
}

fn build(dotnet: &str, solution: &std::path::Path) -> anyhow::Result<()> {
    let status = Command::new(dotnet)
        .args(["build", "-c", "Release"])
        .arg(solution)
        .status()?;
    anyhow::ensure!(status.success(), "dotnet build failed: {status}");
    Ok(())
}
