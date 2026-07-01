use core::net::Ipv4Addr;

use cotl::constants::{GATEWAY_IP, PACKET_META_SLOTS};
use edge_dhcp::{
  Options,
  io::DEFAULT_SERVER_PORT,
  server::{Server, ServerOptions},
};
use embassy_net::{
  IpAddress, IpEndpoint, Ipv4Address, Stack,
  udp::{PacketMetadata, UdpSocket},
};

// max simultaneous DHCP leases
const MAX_LEASES: usize = 8;
const DHCP_BUF_SIZE: usize = 576;
const DHCP_CLIENT_PORT: u16 = 68;

#[allow(clippy::large_stack_frames)]
#[embassy_executor::task]
pub async fn dhcp_task(stack: Stack<'static>) -> ! {
  let mut rx_meta = [PacketMetadata::EMPTY; PACKET_META_SLOTS];
  let mut rx_buf = [0u8; DHCP_BUF_SIZE];
  let mut tx_meta = [PacketMetadata::EMPTY; PACKET_META_SLOTS];
  let mut tx_buf = [0u8; DHCP_BUF_SIZE];
  let mut socket = UdpSocket::new(stack, &mut rx_meta, &mut rx_buf, &mut tx_meta, &mut tx_buf);
  socket.bind(DEFAULT_SERVER_PORT).unwrap();

  let mut server = Server::<_, MAX_LEASES>::new_with_et(GATEWAY_IP);

  let mut gw_buf = [Ipv4Addr::UNSPECIFIED; 1];
  let server_options = ServerOptions::new(GATEWAY_IP, Some(&mut gw_buf));

  let mut recv_buf = [0u8; DHCP_BUF_SIZE];
  let mut send_buf = [0u8; DHCP_BUF_SIZE];

  let broadcast = IpEndpoint::new(IpAddress::Ipv4(Ipv4Address::BROADCAST), DHCP_CLIENT_PORT);

  loop {
    let Ok((len, _)) = socket.recv_from(&mut recv_buf).await else {
      continue;
    };

    let Ok(request) = edge_dhcp::Packet::decode(&recv_buf[..len]) else {
      continue;
    };

    let mut opt_buf = Options::buf();
    if let Some(reply) = server.handle_request(&mut opt_buf, &server_options, &request)
      && let Ok(reply_bytes) = reply.encode(&mut send_buf)
    {
      let _ = socket.send_to(reply_bytes, broadcast).await;
    }
  }
}
