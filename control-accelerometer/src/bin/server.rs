use chrono::{DateTime, Local};
use control_accelerometer::constants::{BROADCAST_IP, BROADCAST_PORT};

use csv::ReaderBuilder;
use quinn_udp::{RecvMeta, UdpSockRef, UdpSocketState};
use socket2::{Domain, Protocol, SockAddr, SockRef, Socket, Type};
use std::fs::File;
use std::io::{ErrorKind, IoSliceMut, Write};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;
use tokio::time::sleep;

pub fn listen_to_multicast_ip(multicast_address: SocketAddrV4) -> anyhow::Result<Socket> {
    let domain = Domain::for_address(multicast_address.into());
    let socket = Socket::new(domain, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    SockRef::from(&socket).set_reuse_port(true)?;
    let address: SocketAddr = format!("0.0.0.0:{}", multicast_address.port()).parse()?;
    println!("Binding to UDP socket {:?}", address);
    socket.bind(&SockAddr::from(address))?;
    socket.join_multicast_v4(multicast_address.ip(), &Ipv4Addr::UNSPECIFIED)?;
    socket.set_nonblocking(false)?;
    socket.set_read_timeout(Some(Duration::from_millis(100)))?;
    Ok(socket)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let multi_addr = SocketAddrV4::new(*BROADCAST_IP, *BROADCAST_PORT);
    let socket = listen_to_multicast_ip(multi_addr)?;
    let socket = std::net::UdpSocket::from(socket);
    // name file using human readable timestamp from chronos
    // Get the current datetime with the local timezone
    let now: DateTime<Local> = Local::now();

    // Format the datetime into a string in the specified format
    let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();
    let mut file = File::create(format!("{}.csv", timestamp))?;

    let quinn_socket = UdpSocketState::default();
    let mut buffer_for_receiving_data = [0u8; 2048];
    let mut iov = [IoSliceMut::new(&mut buffer_for_receiving_data)];
    let mut meta = [RecvMeta::default(); 1];
    loop {
        let socket: UdpSockRef = (&socket).into();
        match quinn_socket.recv(socket, &mut iov, &mut meta) {
            Ok(_len) => {
                // get data
                let len = meta[0].len;
                let data = &iov[0][..len];
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
            Err(e) => {
                if e.kind() == ErrorKind::WouldBlock {
                    sleep(Duration::from_micros(500)).await;
                }
            }
        }
    }
}
