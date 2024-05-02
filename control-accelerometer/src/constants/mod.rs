use std::net::Ipv4Addr;

use lazy_static::lazy_static;

lazy_static! {
    pub static ref BROADCAST_IP: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 123).into();
    pub static ref BROADCAST_PORT: u16 = 7645;
}

pub const ANY: &str = "0.0.0.0";
