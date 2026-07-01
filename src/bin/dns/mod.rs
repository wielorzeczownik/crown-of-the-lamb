use core::net::Ipv4Addr;

use cotl::constants::{GATEWAY_IP, PACKET_META_SLOTS};
use embassy_net::{
  Stack,
  udp::{PacketMetadata, UdpSocket},
};
use simple_dns::{CLASS, Packet, ResourceRecord, rdata::RData};

const DNS_QUERY_BUF_SIZE: usize = 256;
const DNS_RESPONSE_BUF_SIZE: usize = 320;
const DNS_PORT: u16 = 53;
const DNS_RECORD_TTL: u32 = 60;

#[allow(clippy::large_stack_frames)]
#[embassy_executor::task]
pub async fn dns_task(stack: Stack<'static>) -> ! {
  let mut rx_meta = [PacketMetadata::EMPTY; PACKET_META_SLOTS];
  let mut rx_buf = [0u8; DNS_QUERY_BUF_SIZE];
  let mut tx_meta = [PacketMetadata::EMPTY; PACKET_META_SLOTS];
  let mut tx_buf = [0u8; DNS_QUERY_BUF_SIZE];
  let mut socket = UdpSocket::new(stack, &mut rx_meta, &mut rx_buf, &mut tx_meta, &mut tx_buf);
  socket.bind(DNS_PORT).unwrap();

  let mut query_buf = [0u8; DNS_QUERY_BUF_SIZE];
  loop {
    if let Ok((packet_len, sender)) = socket.recv_from(&mut query_buf).await {
      let mut response = [0u8; DNS_RESPONSE_BUF_SIZE];
      let response_len = spoof_dns_reply(&query_buf[..packet_len], &mut response, GATEWAY_IP);
      if response_len > 0 {
        let _ = socket
          .send_to(&response[..response_len], sender.endpoint)
          .await;
      }
    }
  }
}

/// Parses a DNS query and builds an A-record response pointing to `reply_ip`.
/// Acts as a captive portal.
/// All domains resolve to the gateway
fn spoof_dns_reply(
  query: &[u8],
  out: &mut [u8; DNS_RESPONSE_BUF_SIZE],
  reply_ip: Ipv4Addr,
) -> usize {
  let Ok(packet) = Packet::parse(query) else {
    return 0;
  };
  let Some(question) = packet.questions.first() else {
    return 0;
  };
  let domain_name = question.qname.clone();

  let mut reply = packet.into_reply();
  reply.answers.push(ResourceRecord::new(
    domain_name,
    CLASS::IN,
    DNS_RECORD_TTL,
    RData::A(reply_ip.into()),
  ));

  let mut buf: &mut [u8] = &mut out[..];
  if reply.write_to(&mut buf).is_err() {
    return 0;
  }
  DNS_RESPONSE_BUF_SIZE - buf.len()
}
