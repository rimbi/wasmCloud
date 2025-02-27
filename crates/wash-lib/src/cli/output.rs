use std::collections::HashMap;

use serde::Deserialize;
use wasmcloud_control_interface::{Host, HostInventory};
use wasmcloud_core::ActorLinks;

/// JSON Output of the `wash start` command
#[derive(Debug, Deserialize)]
pub struct StartCommandOutput {
    pub actor_id: Option<String>,
    pub actor_ref: Option<String>,

    pub provider_id: Option<String>,
    pub provider_ref: Option<String>,
    pub contract_id: Option<String>,
    pub link_name: Option<String>,

    pub host_id: Option<String>,
    pub success: bool,
}

/// JSON Output representation of the `wash stop` command
#[derive(Debug, Deserialize)]
pub struct StopCommandOutput {
    pub host_id: Option<String>,
    pub result: String,

    pub actor_ref: Option<String>,
    pub actor_id: Option<String>,

    pub provider_id: Option<String>,
    pub link_name: Option<String>,
    pub contract_id: Option<String>,
    pub provider_ref: Option<String>,

    pub success: bool,
}

/// JSON output representation of the `wash link query` command
#[derive(Debug, Deserialize)]
pub struct LinkQueryCommandOutput {
    pub links: Vec<HashMap<String, ActorLinks>>,
    pub success: bool,
}

/// JSON output representation of the `wash get hosts` command
#[derive(Debug, Clone, Deserialize)]
pub struct GetHostsCommandOutput {
    pub success: bool,
    pub hosts: Vec<Host>,
}

/// JSON output representation of the `wash get inventory` command
#[derive(Debug, Clone, Deserialize)]
pub struct GetHostInventoriesCommandOutput {
    pub success: bool,
    pub inventories: Vec<HostInventory>,
}

/// JSON output representation of the `wash get claims` command
#[derive(Debug, Deserialize)]
pub struct GetClaimsCommandOutput {
    pub claims: Vec<HashMap<String, String>>,
    pub success: bool,
}

/// JSON output representation of the `wash dev` command
#[derive(Debug, Deserialize)]
pub struct DevCommandOutput {
    pub success: bool,
}

/// JSON output representation of the `wash dev` command
#[derive(Debug, Deserialize)]
pub struct ScaleCommandOutput {
    pub success: bool,
    pub result: String,
}

/// JSON output representation of the `wash call` command
#[derive(Debug, Deserialize)]
pub struct CallCommandOutput {
    pub success: bool,
    pub response: serde_json::Value,
}
