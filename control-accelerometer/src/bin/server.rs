use control_accelerometer::constants::{ANY, BROADCAST_IP, BROADCAST_PORT};
use csv::ReaderBuilder;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::fs::File;
use std::io::{self, Write};
use std::net::{Ipv4Addr, SocketAddrV4};

fn main() -> io::Result<()> {

    let mcast_group: Ipv4Addr = *BROADCAST_IP;
    let any: Ipv4Addr = *ANY;

    // Create a socket and configure it for reuse
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;

    // Bind the socket to the appropriate port and any interface
    let bind_addr: SocketAddrV4 = SocketAddrV4::new("0.0.0.0".parse().unwrap(), *BROADCAST_PORT);
    let bind_addr = SockAddr::from(bind_addr);
    socket.bind(&bind_addr)?;

    // Join the multicast group
    socket.join_multicast_v4(&mcast_group, &any)?;

    // Convert to std::net::UdpSocket to use in a more familiar API
    let std_socket = std::net::UdpSocket::from(socket);

    let mut file = File::create("output.csv")?;
    let mut buf = [0u8; 10240]; // Buffer size can be adjusted as needed

    loop {
        let (amt, _) = std_socket.recv_from(&mut buf)?;
        println!("got udp data");
        let data = &buf[..amt];
        let cursor = std::io::Cursor::new(data);
        let mut rdr = ReaderBuilder::new().from_reader(cursor);

        for result in rdr.records() {
            let record = result?;
            let csv_string = format!(
                "{},{}\n",
                record.get(0).unwrap_or(""),
                record.get(1).unwrap_or("")
            );
            file.write_all(csv_string.as_bytes())?;
        }
    }
}
