use chrono::Utc;
use csv::Writer;
use std::error::Error;
use std::fs::File;
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let socket = UdpSocket::bind("224.0.0.123:7645").await?;
    socket.join_multicast_v4("224.0.0.123".parse()?, "0.0.0.0".parse()?)?;

    let start_time = Utc::now().format("%Y-%m-%d-%H-%M-%S").to_string();
    let file = File::create(format!("{}.csv", start_time))?;
    let mut wtr = Writer::from_writer(file);

    let mut buf = [0; 1024];
    loop {
        let (len, _) = socket.recv_from(&mut buf).await?;
        let msg = String::from_utf8_lossy(&buf[..len]);
        wtr.write_record(&[msg.as_ref()])?;
        wtr.flush()?;
    }
}
