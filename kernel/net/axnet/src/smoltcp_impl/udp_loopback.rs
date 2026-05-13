use alloc::{collections::VecDeque, sync::Arc, vec::Vec};
use core::net::SocketAddr;

use axsync::Mutex;
use smoltcp::wire::IpEndpoint;

const PORT_NUM: usize = 65536;
const LOOPBACK_UDP_QUEUE_LIMIT: usize = 1024;

#[derive(Clone)]
pub struct UdpLoopbackQueue {
    queue: Arc<Mutex<VecDeque<UdpLoopbackPacket>>>,
}

impl UdpLoopbackQueue {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn push(&self, packet: UdpLoopbackPacket) -> bool {
        let mut queue = self.queue.lock();
        if queue.len() < LOOPBACK_UDP_QUEUE_LIMIT {
            queue.push_back(packet);
            true
        } else {
            false
        }
    }

    pub fn pop_matching(&self, remote: Option<IpEndpoint>) -> Option<UdpLoopbackPacket> {
        let mut queue = self.queue.lock();
        let pos = queue
            .iter()
            .position(|packet| remote.map_or(true, |remote| endpoint_matches(remote, packet.peer)));
        pos.and_then(|pos| queue.remove(pos))
    }

    pub fn has_packet(&self) -> bool {
        !self.queue.lock().is_empty()
    }
}

pub struct UdpLoopbackPacket {
    pub data: Vec<u8>,
    pub peer: IpEndpoint,
}

#[derive(Clone)]
struct UdpLoopbackBinding {
    local: IpEndpoint,
    queue: UdpLoopbackQueue,
}

static UDP_LOOPBACK_TABLE: Mutex<Vec<Option<Vec<UdpLoopbackBinding>>>> = Mutex::new(Vec::new());

pub fn is_loopback_endpoint(endpoint: IpEndpoint) -> bool {
    SocketAddr::from(endpoint).ip().is_loopback()
}

pub fn loopback_source_endpoint(local: IpEndpoint, remote: IpEndpoint) -> IpEndpoint {
    if local.addr.is_unspecified() && is_loopback_endpoint(remote) {
        IpEndpoint::from(SocketAddr::new(SocketAddr::from(remote).ip(), local.port))
    } else {
        local
    }
}

pub fn register_udp_loopback(local: IpEndpoint, queue: UdpLoopbackQueue) {
    let mut table = UDP_LOOPBACK_TABLE.lock();
    if table.is_empty() {
        table.resize_with(PORT_NUM, || None);
    }
    let bindings = table[local.port as usize].get_or_insert_with(Vec::new);
    bindings.push(UdpLoopbackBinding { local, queue });
}

pub fn unregister_udp_loopback(local: IpEndpoint, queue: &UdpLoopbackQueue) {
    let mut table = UDP_LOOPBACK_TABLE.lock();
    if table.is_empty() {
        return;
    }
    if let Some(bindings) = &mut table[local.port as usize] {
        bindings.retain(|binding| {
            binding.local != local || !Arc::ptr_eq(&binding.queue.queue, &queue.queue)
        });
        if bindings.is_empty() {
            table[local.port as usize] = None;
        }
    }
}

pub fn send_udp_loopback(local: IpEndpoint, remote: IpEndpoint, buf: &[u8]) -> usize {
    let table = UDP_LOOPBACK_TABLE.lock();
    if table.is_empty() {
        return buf.len();
    }
    if let Some(bindings) = &table[remote.port as usize] {
        let peer = loopback_source_endpoint(local, remote);
        for binding in bindings {
            if binding_accepts(binding.local, remote) {
                binding.queue.push(UdpLoopbackPacket {
                    data: buf.to_vec(),
                    peer,
                });
            }
        }
    }
    buf.len()
}

fn binding_accepts(local: IpEndpoint, remote: IpEndpoint) -> bool {
    local.port == remote.port && (local.addr.is_unspecified() || local.addr == remote.addr)
}

fn endpoint_matches(expected: IpEndpoint, actual: IpEndpoint) -> bool {
    (expected.addr.is_unspecified() || expected.addr == actual.addr)
        && (expected.port == 0 || expected.port == actual.port)
}
