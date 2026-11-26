use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentRecord {
    pub id: String,
    pub name: String,
    pub version: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRecord {
    pub id: String,
    pub deployment_id: String,
    pub state: String,
    pub service_name: String,
    pub image: String,
    pub ip_address: Option<String>,
    pub port: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct  ContainerRecord {
    pub id: String,
    pub state: String,
    pub container_name: String,
    pub image: String,
    pub ip_address: Option<String>,
    pub port: Option<String>,
}

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct PortForwardRecord {
//     pub id: i64,
//     pub owner_id: String,
//     pub owner_type: String,
//     pub protocol: String,
//     pub host_ip: String,
//     pub host_port: u16,
//     pub container_port: u16,
// }
