//! O6 的探测层：手写 RFC 5389 binding request，向两个独立的 STUN 取反射地址（srflx）。
//!
//! 判定逻辑不在这里——它在 `domain/udp_egress.rs`，输入只是「几个 STUN 答上来了」。
//! Web 侧经浏览器的 WebRTC 栈取同一个东西，两端因此可能得出不同结果（契约 5.6），
//! 这条缝让那处差异只落在探测层。
//!
//! **不引 crate、不引 tokio**：一个 20 字节的请求加一次属性遍历，比任何依赖都短。

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs, UdpSocket};
use std::thread;
use std::time::{Duration, Instant};

/// 两个互相独立的 STUN。对照是必需的：单个 STUN 分不清「UDP 泄露」与「多地址出口集群」。
const SERVERS: [&str; 2] = ["stun.cloudflare.com:3478", "stun.l.google.com:19302"];

const BINDING_REQUEST: u16 = 0x0001;
const BINDING_SUCCESS: u16 = 0x0101;
const MAGIC_COOKIE: u32 = 0x2112_A442;
const XOR_MAPPED_ADDRESS: u16 = 0x0020;
const HEADER_LEN: usize = 20;

/// 96 bit。RFC 5389 用它把应答与请求配对——UDP 无连接，收到的可能是别人的包。
type TransactionId = [u8; 12];

fn transaction_id() -> TransactionId {
    let mut id = [0u8; 12];
    id[..8].copy_from_slice(&super::random_u64().to_be_bytes());
    id[8..].copy_from_slice(&super::random_u64().to_be_bytes()[..4]);
    id
}

/// 20 字节的 binding request：type + length(0) + magic cookie + transaction ID，无属性。
fn binding_request(tid: &TransactionId) -> [u8; HEADER_LEN] {
    let mut message = [0u8; HEADER_LEN];
    message[0..2].copy_from_slice(&BINDING_REQUEST.to_be_bytes());
    // [2..4] 是属性区长度，请求不带属性因此恒为 0。
    message[4..8].copy_from_slice(&MAGIC_COOKIE.to_be_bytes());
    message[8..20].copy_from_slice(tid);
    message
}

fn be_u16(bytes: &[u8]) -> Option<u16> {
    Some(u16::from_be_bytes(bytes.get(..2)?.try_into().ok()?))
}

/// 解一条 binding success response，取出 `XOR-MAPPED-ADDRESS`。
///
/// 任何不合格的输入都返回 `None`：不是成功应答、cookie 不对、**transaction ID 不匹配**、
/// 长度对不上、没有这个属性。其中 transaction ID 这条是安全性要求而不是健壮性要求——
/// 收到别人的应答却拿去比对，等于把一个陌生人的出口地址当成自己的。
pub fn parse_binding_response(message: &[u8], tid: &TransactionId) -> Option<IpAddr> {
    if be_u16(message)? != BINDING_SUCCESS {
        return None;
    }
    let body_len = be_u16(&message[2..])? as usize;
    if u32::from_be_bytes(message.get(4..8)?.try_into().ok()?) != MAGIC_COOKIE {
        return None;
    }
    if message.get(8..HEADER_LEN)? != tid {
        return None;
    }

    let body = message.get(HEADER_LEN..HEADER_LEN + body_len)?;

    let mut offset = 0;
    while offset + 4 <= body.len() {
        let attribute = be_u16(&body[offset..])?;
        let length = be_u16(&body[offset + 2..])? as usize;
        let value = body.get(offset + 4..offset + 4 + length)?;

        if attribute == XOR_MAPPED_ADDRESS {
            return decode_xor_mapped_address(value, tid);
        }
        // 属性值按 4 字节对齐补零，长度字段不含补位。
        offset += 4 + length.next_multiple_of(4);
    }

    None
}

/// `XOR-MAPPED-ADDRESS` 的值：1 字节保留 + 1 字节协议族 + 2 字节异或端口 + 异或后的地址。
///
/// 端口在本判定里用不到（只比 IP），因此只解地址：IPv4 与 magic cookie 异或，
/// IPv6 与 cookie ‖ transaction ID 异或。
fn decode_xor_mapped_address(value: &[u8], tid: &TransactionId) -> Option<IpAddr> {
    let mut mask = [0u8; 16];
    mask[..4].copy_from_slice(&MAGIC_COOKIE.to_be_bytes());
    mask[4..].copy_from_slice(tid);

    match value.get(1)? {
        0x01 => {
            let mut octets: [u8; 4] = value.get(4..8)?.try_into().ok()?;
            for (octet, key) in octets.iter_mut().zip(mask) {
                *octet ^= key;
            }
            Some(IpAddr::V4(Ipv4Addr::from(octets)))
        }
        0x02 => {
            let mut octets: [u8; 16] = value.get(4..20)?.try_into().ok()?;
            for (octet, key) in octets.iter_mut().zip(mask) {
                *octet ^= key;
            }
            Some(IpAddr::V6(Ipv6Addr::from(octets)))
        }
        _ => None,
    }
}

/// 向两个 STUN 并发取反射地址。**拿到几个算几个**——列表里只放答上来的
/// （即契约 2.6 的 `N_ok`），没答上来的不占位，判定留给 `domain/udp_egress.rs`。
pub fn probe(timeout: Duration) -> Vec<IpAddr> {
    thread::scope(|scope| {
        let queries: Vec<_> = SERVERS
            .iter()
            .map(|server| scope.spawn(move || query(server, timeout)))
            .collect();

        queries
            .into_iter()
            .filter_map(|query| query.join().ok().flatten())
            .collect()
    })
}

fn query(server: &str, timeout: Duration) -> Option<IpAddr> {
    let address = server.to_socket_addrs().ok()?.next()?;

    // 本地端口必须与目标同族，否则 connect 直接失败。
    let local: SocketAddr = if address.is_ipv4() {
        (Ipv4Addr::UNSPECIFIED, 0).into()
    } else {
        (Ipv6Addr::UNSPECIFIED, 0).into()
    };
    let socket = UdpSocket::bind(local).ok()?;
    // `connect` 让内核只投递来自该地址的包。它**不能**替代 transaction ID 校验：
    // UDP 的源地址是可伪造的，而 96 bit 的随机 ID 不是。
    socket.connect(address).ok()?;

    let tid = transaction_id();
    socket.send(&binding_request(&tid)).ok()?;

    // 收到不匹配的包不算这次探测失败——它可能是上一轮的迟到应答，继续等到超时为止。
    let deadline = Instant::now() + timeout;
    let mut buffer = [0u8; 1024];
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return None;
        }
        socket.set_read_timeout(Some(remaining)).ok()?;

        let received = socket.recv(&mut buffer).ok()?;
        if let Some(ip) = parse_binding_response(&buffer[..received], &tid) {
            return Some(ip);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TID: TransactionId = [
        0xb7, 0xe7, 0xa7, 0x01, 0xbc, 0x34, 0xd6, 0x86, 0xfa, 0x87, 0xdf, 0xae,
    ];

    /// RFC 5769 2.1 的样例响应（截取到 XOR-MAPPED-ADDRESS 属性为止）。
    /// 异或后的 `e1:12:a4:43` 对应 192.0.2.1。
    fn ipv4_response() -> Vec<u8> {
        let mut message = vec![0x01, 0x01, 0x00, 0x0c];
        message.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());
        message.extend_from_slice(&TID);
        message.extend_from_slice(&[0x00, 0x20, 0x00, 0x08]);
        message.extend_from_slice(&[0x00, 0x01, 0xa1, 0x47, 0xe1, 0x12, 0xa6, 0x43]);
        message
    }

    /// RFC 5769 2.4 的样例响应。异或后对应 2001:db8:1234:5678:11:2233:4455:6677。
    fn ipv6_response() -> Vec<u8> {
        let mut message = vec![0x01, 0x01, 0x00, 0x18];
        message.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());
        message.extend_from_slice(&TID);
        message.extend_from_slice(&[0x00, 0x20, 0x00, 0x14]);
        message.extend_from_slice(&[
            0x00, 0x02, 0xa1, 0x47, 0x01, 0x13, 0xa9, 0xfa, 0xa5, 0xd3, 0xf1, 0x79, 0xbc, 0x25,
            0xf4, 0xb5, 0xbe, 0xd2, 0xb9, 0xd9,
        ]);
        message
    }

    #[test]
    fn the_request_is_twenty_bytes_with_the_magic_cookie() {
        let request = binding_request(&TID);
        assert_eq!(request.len(), 20);
        assert_eq!(&request[0..2], &[0x00, 0x01], "type = binding request");
        assert_eq!(&request[2..4], &[0x00, 0x00], "无属性，长度为 0");
        assert_eq!(&request[4..8], &MAGIC_COOKIE.to_be_bytes());
        assert_eq!(&request[8..20], &TID);
    }

    #[test]
    fn transaction_ids_do_not_repeat() {
        // 固定的 ID 会让「收到别人的包」这道校验形同虚设。
        assert_ne!(transaction_id(), transaction_id());
    }

    #[test]
    fn decodes_an_xor_mapped_ipv4_address() {
        assert_eq!(
            parse_binding_response(&ipv4_response(), &TID),
            Some("192.0.2.1".parse().unwrap())
        );
    }

    #[test]
    fn decodes_an_xor_mapped_ipv6_address() {
        assert_eq!(
            parse_binding_response(&ipv6_response(), &TID),
            Some("2001:db8:1234:5678:11:2233:4455:6677".parse().unwrap())
        );
    }

    #[test]
    fn a_mismatched_transaction_id_is_discarded() {
        // UDP 无连接：收到的可能是别人的应答。拿它当自己的反射地址，
        // 等于把一个陌生人的出口地址报给用户。
        let mut other = TID;
        other[0] ^= 0xff;
        assert_eq!(parse_binding_response(&ipv4_response(), &other), None);
    }

    #[test]
    fn malformed_responses_are_discarded_not_guessed() {
        let good = ipv4_response();

        // 截断
        assert_eq!(parse_binding_response(&good[..12], &TID), None);
        // 不是成功应答（这里是 binding request 的 type）
        let mut wrong_type = good.clone();
        wrong_type[0..2].copy_from_slice(&BINDING_REQUEST.to_be_bytes());
        assert_eq!(parse_binding_response(&wrong_type, &TID), None);
        // cookie 不对
        let mut wrong_cookie = good.clone();
        wrong_cookie[4] ^= 0xff;
        assert_eq!(parse_binding_response(&wrong_cookie, &TID), None);
        // 属性长度超出报文
        let mut overlong = good.clone();
        overlong[23] = 0xff;
        assert_eq!(parse_binding_response(&overlong, &TID), None);
    }

    #[test]
    fn skips_other_attributes_including_padded_ones() {
        // 真实响应常先带 SOFTWARE（长度不是 4 的倍数，按 4 字节补零）。
        let mut message = vec![0x01, 0x01, 0x00, 0x18];
        message.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());
        message.extend_from_slice(&TID);
        message.extend_from_slice(&[0x80, 0x22, 0x00, 0x05]);
        message.extend_from_slice(b"hello\0\0\0");
        message.extend_from_slice(&[0x00, 0x20, 0x00, 0x08]);
        message.extend_from_slice(&[0x00, 0x01, 0xa1, 0x47, 0xe1, 0x12, 0xa6, 0x43]);

        assert_eq!(
            parse_binding_response(&message, &TID),
            Some("192.0.2.1".parse().unwrap())
        );
    }

    #[test]
    fn a_response_without_the_attribute_yields_nothing() {
        let mut message = vec![0x01, 0x01, 0x00, 0x08];
        message.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());
        message.extend_from_slice(&TID);
        message.extend_from_slice(&[0x80, 0x22, 0x00, 0x04]);
        message.extend_from_slice(b"none");

        assert_eq!(parse_binding_response(&message, &TID), None);
    }
}
