use ipnet::IpNet;
use std::str::FromStr;

pub fn address_parser(string: &str) -> Result<IpNet, String> {
    IpNet::from_str(string).map_err(|err| err.to_string())
}

pub fn parse_to_p2p_nets(cidrs_strings: &Vec<String>) -> Vec<IpNet> {
    cidrs_strings
        .iter()
        .flat_map(|cidr| {
            IpNet::from_str(cidr)
                .unwrap()
                .subnets(30)
                .unwrap()
                .collect::<Vec<IpNet>>()
        })
        .collect()
}

pub mod models;
