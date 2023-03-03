mod cidrs_utils;

use crate::cidrs_utils::{address_parser, models::{NegotiationRequest, NegotiationResponse}};
use std::{net::IpAddr, process::Command};

use cidrs_utils::parse_to_p2p_nets;
use clap::Parser;
use ipnet::IpNet;
use log::info;
use reqwest::Client;
use rocket::{
    http::Status,
    response::status::NotFound,
    serde::{json::Json, Deserialize, Serialize},
};

#[macro_use]
extern crate rocket;

static mut LOCAL_P2P_NETS: Vec<IpNet> = Vec::new();
static mut LOCAL_CIDRS: Vec<IpNet> = Vec::new();
static mut INTERNAL_LOCAL_NETWORK: Option<IpNet> = None;
static mut INTERFACE: Option<String> = None;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[clap(short, long, value_parser = address_parser)]
    cidrs: Vec<IpNet>,

    #[clap(short, long)]
    internal_network: Option<IpNet>,

    #[clap(long)]
    interface: Option<String>
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct NegotiationInformation {
    endpoint: String,
    destination_network: Option<IpNet>
}

#[post("/handle_negotiation", data = "<remote_cidrs>")]
pub fn handle_negotiation(remote_cidrs: Json<NegotiationRequest>) -> Result<Json<NegotiationResponse>, NotFound<String>> {
    info!("Negotiation request received with the following proposed CIDRs: {:?}", remote_cidrs);

    let safe_local_p2p_nets = unsafe { LOCAL_P2P_NETS.clone() };
    let remotes: Vec<IpNet> = parse_to_p2p_nets(&remote_cidrs.cidrs);

    info!("Checking if a possible /30 net exists");
    for remote in remotes {
        if safe_local_p2p_nets.contains(&remote) {
            info!("{} is also available locally so it is chosen as p2p network between the two routers", remote);
            let hosts = remote.hosts().collect::<Vec<IpAddr>>();
            let ip_to_assign_locally = hosts.get(0).unwrap();
            let free_ip = hosts.get(1).unwrap();

            info!("Configuring the router");
            Command::new("ip")
                .arg("addr")
                .arg("add")
                .arg(format!("{}/30", ip_to_assign_locally.to_string()))
                .arg("dev")
                .arg(unsafe { INTERFACE.clone() }.unwrap_or(String::from("enp0s2")))
                .spawn()
                .unwrap();

            if let Some(destination_network) = &remote_cidrs.destination_network {
                Command::new("ip")
                .arg("route")
                .arg("add")
                .arg(destination_network.to_string())
                .arg("via")
                .arg(free_ip.to_string())
                .spawn()
                .unwrap();
            }
            info!("Done! Router configured");

            let safe_internal_network = unsafe { INTERNAL_LOCAL_NETWORK.clone() };

            return Ok(rocket::serde::json::Json(NegotiationResponse::new(remote, *free_ip, *ip_to_assign_locally, safe_internal_network)));
        }
    }

    info!("No commong /30 network found");
    Err(NotFound(String::from("No common CIDR found")))
}

#[post("/start_negotiation", data = "<remote_agent>")]
pub async fn start_negotiation(remote_agent: Json<NegotiationInformation>) -> Status {
    info!("A request for starting a negotiation session has been received with the request: {:?}", remote_agent);
    let safe_local_cidrs = unsafe { LOCAL_CIDRS.clone() };

    info!("CIDRs currently available locally are: {:?}. Sending them as a proposal", safe_local_cidrs);
    let client = Client::new();

    let cidrs: Vec<String> = safe_local_cidrs
        .iter()
        .map(|cidr| cidr.to_string())
        .collect();

    let response = client
        .post(&remote_agent.endpoint)
        .json(&NegotiationRequest::new(cidrs, remote_agent.destination_network))
        .send()
        .await
        .unwrap();

    if response.status().is_success() {
        let b: NegotiationResponse = response.json().await.unwrap();
        info!("A response has been received with success with the response: {:?}", b);

        Command::new("ip")
            .arg("addr")
            .arg("add")
            .arg(format!("{}/30", b.free_ip.to_string()))
            .arg("dev")
            .arg(unsafe { INTERFACE.clone() }.unwrap_or(String::from("enp0s2")))
            .spawn()
            .unwrap();

        if let Some(destination_network) = &b.destination_network {
            info!("The request contains a destination retwork, configuring a route for it");
            Command::new("ip")
                .arg("route")
                .arg("add")
                .arg(destination_network.to_string())
                .arg("via")
                .arg(b.assigned_ip.to_string())
                .spawn()
                .unwrap();
        }

        Status::Ok
    } else {
        Status::InternalServerError
    }
}

pub fn parse_args() {
    let cli = Cli::parse();

    let point_to_point_nets: Vec<IpNet> = cli
        .cidrs
        .iter()
        .flat_map(|cidr| cidr.subnets(30).unwrap().collect::<Vec<IpNet>>())
        .collect();

    if let Some(internal_network) = cli.internal_network {
        unsafe { INTERNAL_LOCAL_NETWORK = Some(internal_network); }
    }

    if let Some(interface) = cli.interface {
        unsafe { INTERFACE = Some(interface) };
    }

    info!("Args parsed. Internal Network: {:?}, CIDRs: {:?}", cli.internal_network, cli.cidrs);
    unsafe {
        LOCAL_CIDRS = cli.cidrs;
        LOCAL_P2P_NETS = point_to_point_nets;
    }
}
