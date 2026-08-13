//! `preflight dns --check` 的 UDP 实测。
//!
//! 向每台 DNS 的 UDP 53 发一条 A 记录查询，按 spec §4.3 判据判定状态：
//! - 通：收到应答，TXID 匹配，RCODE = NOERROR，ANCOUNT > 0，A 记录非私网。
//! - 应答可疑：有应答但不满足上述全部条件。
//! - 不通：超时无应答（或收到 TXID 不匹配的包后继续等待直到超时）。
//!
//! 解析用 `simple-dns`（spec §4.5），不手写——DNS 名称压缩指针是手写解析器的
//! 经典翻车点，而这里解析的恰恰是我们正在怀疑其被劫持的服务器返回的输入。

use std::net::Ipv4Addr;
use std::net::UdpSocket;
use std::time::{Duration, Instant};

use simple_dns::{CLASS, Name, Packet, QCLASS, QTYPE, Question, RCODE, TYPE, rdata::RData};

/// 一台 DNS 服务器的实测状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ok,
    Suspicious,
    Unreachable,
}

/// 一台 DNS 服务器的实测结果。
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub status: Status,
    /// 仅在收到有效应答时有值。
    pub latency: Option<Duration>,
}

/// 查询的域名——任何公共 DNS 都能解析的稳定域名。
const QUERY_NAME: &str = "example.com";

/// 构建一条 A 记录查询。
fn build_query(txid: u16) -> Vec<u8> {
    let mut packet = Packet::new_query(txid);
    let qname = Name::new(QUERY_NAME).expect("域名常量必须合法");
    packet.questions.push(Question::new(
        qname,
        QTYPE::TYPE(TYPE::A),
        QCLASS::CLASS(CLASS::IN),
        false,
    ));
    packet
        .build_bytes_vec_compressed()
        .expect("构建 DNS 查询不应失败")
}

/// 解析应答并判定状态。
///
/// 返回 `None` 表示 TXID 不匹配——调用方应丢弃并继续等待。
fn judge_response(expected_txid: u16, response: &[u8]) -> Option<Status> {
    let packet = Packet::parse(response).ok()?;

    // 判据 1：TXID 必须匹配。不匹配 = 丢弃。
    if packet.id() != expected_txid {
        return None;
    }

    // 判据 2：RCODE = NOERROR 且 ANCOUNT > 0。
    if packet.rcode() != RCODE::NoError || packet.answers.is_empty() {
        return Some(Status::Suspicious);
    }

    // 判据 3：返回的 A 记录不能是私网地址。
    let all_public = packet.answers.iter().all(|rr| match &rr.rdata {
        RData::A(a) => {
            let addr = Ipv4Addr::from(a.address);
            !addr.is_private() && !addr.is_loopback() && !addr.is_link_local()
        }
        _ => true, // 非 A 记录不参与判定
    });

    if all_public {
        Some(Status::Ok)
    } else {
        Some(Status::Suspicious)
    }
}

/// 实测一台 DNS 服务器。
pub fn check(server_ip: &str, timeout: Duration) -> CheckResult {
    let Ok(socket) = UdpSocket::bind("0.0.0.0:0") else {
        return CheckResult {
            status: Status::Unreachable,
            latency: None,
        };
    };
    let _ = socket.set_read_timeout(Some(timeout));

    // 16 位 TXID，用进程内递增计数 + 时间戳混合，不追求密码学强度。
    let txid = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_micros())
        .unwrap_or(0)
        % 0xFFFF) as u16;

    let query = build_query(txid);
    let start = Instant::now();

    let Ok(_) = socket.send_to(&query, (server_ip, 53)) else {
        return CheckResult {
            status: Status::Unreachable,
            latency: None,
        };
    };

    let mut buf = [0u8; 512];
    loop {
        match socket.recv_from(&mut buf) {
            Ok((len, _)) => {
                if let Some(status) = judge_response(txid, &buf[..len]) {
                    return CheckResult {
                        status,
                        latency: Some(start.elapsed()),
                    };
                }
                // TXID 不匹配：丢弃，继续等待。
            }
            Err(_) => {
                // 超时或 IO 错误。
                return CheckResult {
                    status: Status::Unreachable,
                    latency: None,
                };
            }
        }
    }
}

/// 并发实测全表。用 `std::thread::scope`，不引 tokio（spec §4.5）。
/// 结果顺序与输入顺序一致。
pub fn check_all(
    entries: &[crate::domain::dns_servers::Entry],
    timeout: Duration,
) -> Vec<CheckResult> {
    std::thread::scope(|scope| {
        let handles: Vec<_> = entries
            .iter()
            .map(|entry| scope.spawn(move || check(&entry.ip, timeout)))
            .collect();
        handles
            .into_iter()
            .map(|h| {
                h.join().unwrap_or(CheckResult {
                    status: Status::Unreachable,
                    latency: None,
                })
            })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use simple_dns::{ResourceRecord, rdata::A};

    /// 构建一个 DNS 应答包用于测试。
    fn build_response(txid: u16, rcode: RCODE, answers: Vec<(Ipv4Addr,)>) -> Vec<u8> {
        let mut packet = Packet::new_reply(txid);
        *packet.rcode_mut() = rcode;
        let name = Name::new(QUERY_NAME).unwrap();
        for (addr,) in answers {
            packet.answers.push(ResourceRecord::new(
                name.clone(),
                CLASS::IN,
                300,
                RData::A(A::from(addr)),
            ));
        }
        packet.build_bytes_vec_compressed().unwrap()
    }

    #[test]
    fn noerror_with_public_a_is_ok() {
        let resp = build_response(
            0x1234,
            RCODE::NoError,
            vec![(Ipv4Addr::new(93, 184, 216, 34),)],
        );
        assert_eq!(judge_response(0x1234, &resp), Some(Status::Ok));
    }

    #[test]
    fn nxdomain_is_suspicious() {
        let resp = build_response(0x1234, RCODE::NameError, vec![]);
        assert_eq!(judge_response(0x1234, &resp), Some(Status::Suspicious));
    }

    #[test]
    fn private_a_record_is_suspicious() {
        let resp = build_response(
            0x1234,
            RCODE::NoError,
            vec![(Ipv4Addr::new(192, 168, 1, 1),)],
        );
        assert_eq!(judge_response(0x1234, &resp), Some(Status::Suspicious));
    }

    #[test]
    fn txid_mismatch_is_discarded() {
        let resp = build_response(
            0x5678,
            RCODE::NoError,
            vec![(Ipv4Addr::new(93, 184, 216, 34),)],
        );
        assert_eq!(judge_response(0x1234, &resp), None);
    }

    #[test]
    fn no_answers_is_suspicious() {
        // RCODE = NOERROR 但 ANCOUNT = 0：有应答但没有 A 记录。
        let resp = build_response(0x1234, RCODE::NoError, vec![]);
        assert_eq!(judge_response(0x1234, &resp), Some(Status::Suspicious));
    }

    #[test]
    fn loopback_a_record_is_suspicious() {
        let resp = build_response(0x1234, RCODE::NoError, vec![(Ipv4Addr::new(127, 0, 0, 1),)]);
        assert_eq!(judge_response(0x1234, &resp), Some(Status::Suspicious));
    }

    #[test]
    fn malformed_packet_is_discarded() {
        // 垃圾字节：解析失败 = 丢弃。
        assert_eq!(judge_response(0x1234, &[0xFF, 0xFF, 0xFF]), None);
    }
}
