use std::net::Ipv4Addr;
use std::str::FromStr;

pub fn get_ipv4_addresses(all_ips: Vec<String>) -> Vec<String> {
    let mut result = Vec::<String>::new();
    for ip in all_ips {
        if Ipv4Addr::from_str(&ip).is_ok() {
            result.push(ip);
        }
    }

    result
}
