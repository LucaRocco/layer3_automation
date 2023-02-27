use std::{net::IpAddr, str::FromStr, time::Duration, process::Command};

use clap::Parser;
use ipnet::IpNet;
use reqwest::{ClientBuilder};
use rocket::{
    http::{Status, hyper::body},
    response::status::NotFound,
    serde::{json::Json, Deserialize, Serialize},
};

#[macro_use]
extern crate rocket;

static mut LOCAL_P2P_NETS: Vec<IpNet> = Vec::new();
static mut LOCAL_CIDRS: Vec<IpNet> = Vec::new();

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[clap(short, long, value_parser = address_parser)]
    cidrs: Vec<IpNet>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(crate = "rocket::serde")]
struct CIDRs {
    cidrs: Vec<String>,
}

impl CIDRs {
    fn new(cidrs: Vec<String>) -> Self {
        CIDRs { cidrs }
    }
}

#[derive(Deserialize, Serialize, Debug, Copy, Clone)]
#[serde(crate = "rocket::serde")]
struct CIDR {
    net: IpNet,
    free_ip: IpAddr,
}

impl CIDR {
    fn new(net: IpNet, free_ip: IpAddr) -> Self {
        CIDR { net, free_ip }
    }
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(crate = "rocket::serde")]
struct RemoteAgent {
    endpoint: String,
}

#[post("/handle_negotiation", data = "<remote_cidrs>")]
fn handle_negotiation(remote_cidrs: Json<CIDRs>) -> Result<Json<CIDR>, NotFound<String>> {
    let safe_local_p2p_nets = unsafe { LOCAL_P2P_NETS.clone() };

    let remotes: Vec<IpNet> = remote_cidrs
        .cidrs
        .iter()
        .flat_map(|cidr| {
            IpNet::from_str(cidr)
                .unwrap()
                .subnets(30)
                .unwrap()
                .collect::<Vec<IpNet>>()
        })
        .collect();

    for remote in remotes {
        if safe_local_p2p_nets.contains(&remote) {
            let hosts = remote.hosts().collect::<Vec<IpAddr>>();
            let _ip_to_assign_locally = hosts.get(0).unwrap();
            let free_ip = hosts.get(1).unwrap();

            return Ok(rocket::serde::json::Json(CIDR::new(remote, *free_ip)));
        }
    }

    Err(NotFound(String::from("No common CIDR found")))
}

#[post("/start_negotiation", data = "<remote_agent>")]
async fn start_negotiation(remote_agent: Json<RemoteAgent>) -> Status {

    let safe_local_cidrs = unsafe { LOCAL_CIDRS.clone() };

    let timeout = Duration::new(5, 0);
    let client = ClientBuilder::new().timeout(timeout).build().unwrap();
    let cidrs: Vec<String> = safe_local_cidrs.iter().map(|cidr| { cidr.to_string() }).collect();
    let response = client
        .post(&remote_agent.endpoint)
        .json(&CIDRs::new(cidrs))
        .send()
        .await.unwrap();

    if response.status().is_success() {
        let b: CIDR = response.json().await.unwrap();
        println!("{:?}", b);

        Command::new("ip").arg("addr").arg("add").arg(format!("{}/{}", b.free_ip.to_string(), b.net.prefix_len())).arg("dev").arg("enp0s2").spawn().unwrap();
        Status::Ok
    } else {
        Status::InternalServerError
    }
}

#[launch]
fn rocket() -> _ {
    let cli = Cli::parse();

    let point_to_point_nets: Vec<IpNet> = cli
        .cidrs
        .iter()
        .flat_map(|cidr| cidr.subnets(30).unwrap().collect::<Vec<IpNet>>())
        .collect();

    println!("{:?}", point_to_point_nets);

    unsafe {
        LOCAL_CIDRS = cli.cidrs;
        LOCAL_P2P_NETS = point_to_point_nets;
    }

    rocket::build().mount("/", routes![handle_negotiation, start_negotiation])
}

fn address_parser(string: &str) -> Result<IpNet, String> {
    IpNet::from_str(string).map_err(|err| err.to_string())
}
