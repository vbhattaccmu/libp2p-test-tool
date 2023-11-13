//! Basic tests to demonstrate results of Task 1, 2 and 3
//! with the help of this tool.

use csv::{Reader, StringRecord};
use std::{
    env,
    fs::File,
    process::Command,
    time::{Duration, Instant},
};
use tokio::time::sleep;

#[tokio::test(flavor = "multi_thread")]
async fn test_unreachable_peer_log() {
    let current_path = env::current_dir().unwrap();
    let results_dir = "/results/test_unreachable_peer_log";
    env::set_var("VOLUME_PATH", &results_dir);

    let compose = DockerCompose::new("tests/compose/avail-setup-with-tool.yml");
    let _ = DockerCompose::serve(&compose);

    sleep(Duration::from_secs(SLEEP)).await;

    // If we disconnect the avail-light-1-client from the network,
    // it wont be dialable any longer, since its udp port is not exposed in compose
    compose
        .disconnect("avail-light-1", NETWORK_NAME)
        .expect("Container disconnect failed");

    // Let the tool operate throughout its whole duration
    sleep(Duration::from_secs(OPERATION_DURATION)).await;

    compose.clean_up().expect("Failed to clean up services");

    // Read CSV file for unreachable peers
    let unreachable: File = File::open(format!(
        "{}/tests/compose{}/unreachable.csv",
        current_path.to_string_lossy(),
        results_dir
    ))
    .unwrap();

    let mut is_present: bool = false;
    let mut reader = Reader::from_reader(unreachable);

    // `avail-light-1-client` multiaddr should be in `unreachable.csv` records
    for record in reader.records() {
        let record = StringRecord::from(record.unwrap());
        let multi_addr = &record[0];

        if multi_addr == DISCONNECTED_PEER_MULTIADDRESS {
            is_present = true;
        }
    }

    assert_eq!(is_present, true);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_new_peer_join_and_ip_resolution() {
    let current_path = env::current_dir().unwrap();
    let results_dir = "/results/test_new_peer_join_and_ip_resolution";
    env::set_var("VOLUME_PATH", &results_dir);

    let compose = DockerCompose::new("tests/compose/avail-setup-with-tool.yml");
    let _ = DockerCompose::serve(&compose);

    sleep(Duration::from_secs(SLEEP)).await;

    // Add new peer to existing network
    compose
        .add_external_light_client("avail-light-3")
        .expect("could not add external peer image to existing network.");

    // Let the tool operate throughout its whole duration
    sleep(Duration::from_secs(OPERATION_DURATION)).await;

    compose
        .delete("avail-light-3")
        .expect("Failed to delete container.");
    compose.clean_up().expect("Failed to clean up services");

    // Read newly_connected CSV file for new peers
    let newly_connected: File = File::open(format!(
        "{}/tests/compose{}/newly_connected.csv",
        current_path.to_string_lossy(),
        results_dir
    ))
    .unwrap();

    // Check if the newly added peer exists in logs
    let mut is_present: bool = false;
    let mut reader = Reader::from_reader(newly_connected);

    // Assert new peer addr and IP
    for record in reader.records() {
        let record = StringRecord::from(record.unwrap());
        let peer_id = &record[0];
        let ip_addr = &record[1];

        if peer_id == NEW_PEER_ID && ip_addr == NEW_PEER_OBSERVED_IP {
            is_present = true;
        }
    }

    assert_eq!(is_present, true);

    let unreachable: File = File::open(format!(
        "{}/tests/compose{}/unreachable.csv",
        current_path.to_string_lossy(),
        results_dir
    ))
    .unwrap();

    // unreachable peer count should be 0 (only headers) since everyone
    // is dialable in the network
    assert_eq!(
        Reader::from_reader(unreachable).records().count() == 0,
        true
    );
}

//////////////////////////////////////////////////////////////
// Helpers for setting up/modifying a local network via Docker
//////////////////////////////////////////////////////////////

struct DockerCompose {
    yaml_path: String,
}

impl DockerCompose {
    // Create a new instance of compose
    pub fn new(yaml_path: &str) -> Self {
        let current_path = env::current_dir().unwrap();
        let docker_compose_path = format!("{}/{}", current_path.to_str().unwrap(), yaml_path);

        DockerCompose {
            yaml_path: docker_compose_path,
        }
    }

    // Run any general command, requires docker daemon to be active
    fn run_command(command: &str, args: &[&str]) -> Result<String, String> {
        let (path_to_shell, option, _suppress_output) = match cfg!(target_os = "windows") {
            false => ("/bin/bash", "-c", ">/dev/null"),
            true => ("cmd", "/C", ">nul"),
        };

        let mut merged_cmd = Vec::new();
        merged_cmd.push(command);
        merged_cmd.extend(args);

        let output = Command::new(path_to_shell)
            .arg(option)
            .arg(&merged_cmd.join(" "))
            .output()
            .expect("failed to execute process");

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    // Start compose
    pub fn serve(compose: &DockerCompose) -> Result<String, String> {
        while compose.no_running_containers_in_service(Duration::from_secs(300)) {
            if compose.up().is_err() {
                return Err(String::from("Resource busy"));
            } else {
                break;
            }
        }

        Ok(String::from("Success"))
    }

    // Check if any container is running from the spawned stack
    fn no_running_containers_in_service(&self, timeout: Duration) -> bool {
        let start_time = Instant::now();
        loop {
            let containers = Self::run_command(
                "docker-compose",
                &["-f", &self.yaml_path, "ps", "--status", "running"],
            )
            .unwrap();

            if containers.matches('\n').count() <= 1 {
                return true;
            }
            if start_time.elapsed() > timeout {
                return false;
            }
        }
    }

    // Set up compose
    fn up(&self) -> Result<String, String> {
        Self::run_command("docker-compose", &["-f", &self.yaml_path, "up", "-d"])
    }

    // Add a predefined external peer into the network
    fn add_external_light_client(&self, container: &str) -> Result<String, String> {
        Self::run_command("docker", 
        &[
            "run", "-d",
            "--name", container,
            "-e", "LC_LOG_LEVEL=INFO",
            "-e", "LC_HTTP_SERVER_HOST=172.16.3.5",
            "-e", "LC_HTTP_SERVER_PORT=7000",
            "-e", "LC_SECRET_KEY_SEED=3",
            "-e", "LC_LIBP2P_SEED=1",
            "-e", "LC_LIBP2P_PORT=37000",
            "-e", "LC_FULL_NODE_RPC=http://172.16.3.1:9933",
            "-e", "LC_FULL_NODE_WS=ws://172.16.3.1:9944",
            "-e", "LC_APP_ID=1",
            "-e", "LC_CONFIDENCE=92.0",
            "-e", "LC_AVAIL_PATH=/da/state",
            "-e", "LC_PROMETHEUS_PORT=9520",
            "-e", "LC_BOOTSTRAPS=/ip4/172.16.3.2/udp/39000/quic-v1/p2p/12D3KooWStAKPADXqJ7cngPYXd2mSANpdgh1xQ34aouufHA2xShz",
            "-p", "37002:37000",
            "-p", "9522:9520",
            "-p", "7002:7000",
            "--network", "compose_net",
            "--ip", "172.16.3.5",
            "--restart", "on-failure",
            "vbhattac/avail-light:latest",
        ])
    }

    // Disconnect a container from network
    fn disconnect(&self, container_name: &str, network: &str) -> Result<String, String> {
        Self::run_command(
            "docker",
            &["network", "disconnect", network, container_name],
        )
    }

    // Cleanup network artifacts
    fn clean_up(&self) -> Result<String, String> {
        Self::run_command("docker-compose", &["-f", &self.yaml_path, "down", "-v"])
    }

    // Delete a container
    fn delete(&self, container: &str) -> Result<String, String> {
        Self::run_command("docker", &["stop", container])?;
        Self::run_command("docker", &["rm", container])
    }
}

impl Drop for DockerCompose {
    fn drop(&mut self) {
        self.clean_up()
            .expect("Could not clean up artifacts. Please manually remove them.");
    }
}

////////////////////////
// Predefined Constants
////////////////////////

const SLEEP: u64 = 15;
const OPERATION_DURATION: u64 = 181;
const NETWORK_NAME: &str = "compose_net";
const NEW_PEER_OBSERVED_IP: &str = "172.16.3.6";
const NEW_PEER_ID: &str = "12D3KooWEU4Vs8N8X8sSJua4crFGvgii4C7Eusi1L5ukxjfzhpmk";
const DISCONNECTED_PEER_MULTIADDRESS: &str =
    "/ip4/172.16.3.3/udp/37000/quic-v1/p2p/12D3KooWE2xXc6C2JzeaCaEg7jvZLogWyjLsB5dA3iw5o3KcF9ds";
