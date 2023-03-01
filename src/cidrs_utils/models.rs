use std::net::IpAddr;

use ipnet::IpNet;
use rocket::serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct NegotiationRequest {
    pub cidrs: Vec<String>,
    pub destination_network: Option<IpNet>
}

#[derive(Deserialize, Serialize, Debug, Copy, Clone)]
#[serde(crate = "rocket::serde")]
pub struct NegotiationResponse {
    net: IpNet,
    pub free_ip: IpAddr,
    pub destination_network: Option<IpNet>
}

impl NegotiationRequest {
  pub fn new(cidrs: Vec<String>, destination_network: Option<IpNet>) -> Self {
      NegotiationRequest { cidrs, destination_network }
  }
}

impl NegotiationResponse {
  pub fn new(net: IpNet, free_ip: IpAddr, destination_network: Option<IpNet>) -> Self {
      NegotiationResponse { net, free_ip, destination_network }
  }
}