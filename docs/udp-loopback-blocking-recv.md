# UDP loopback blocking receive repair

## Background

The RV tests exercise local UDP traffic through `netserver` and `netperf`.
Before this repair, loopback UDP packets could be delivered into the kernel-side
loopback queue, but a receiver that had already entered a blocking `recv` or
`recvfrom` call did not observe packets that arrived after the call began.

The failure was in the socket implementation, not in the test runner or image.
The evaluation scripts and test images are unchanged.

## Root cause

`UdpSocket::recv_from` checked the loopback queue once before falling into the
normal smoltcp receive wait path. If the queue was empty at entry time, the
blocking loop only rechecked the smoltcp socket. A loopback datagram delivered
later therefore remained queued while the receiver kept waiting on the wrong
readiness source.

This is a general UDP loopback blocking semantics bug. The repair does not use
program names, port numbers, packet sizes, or benchmark-specific conditions.

## Change

`UdpSocket::recv_from` and connected `UdpSocket::recv` now check the loopback
queue and the smoltcp UDP socket in the same blocking wait loop:

1. Try to consume a matching loopback datagram.
2. If no loopback datagram is ready, poll the smoltcp UDP socket.
3. If neither source is ready, preserve the existing nonblocking or timeout
   behavior.

The queue copy logic is shared through `try_recv_loopback_from` so the blocking
and immediate loopback paths use the same packet matching and source-address
handling.

The loopback queue debug counters and temporary diagnostic logs used during
triage were removed before validation.

## Behavior preservation

- No evaluation script, test binary, or image is changed.
- Nonblocking sockets still return `WouldBlock` when neither loopback nor
  smoltcp data is ready.
- Receive timeouts still cover the whole blocking wait.
- Regular smoltcp UDP receive remains available when no matching loopback
  datagram is ready.
- The source address returned by `recvfrom` remains the datagram peer endpoint.
- The implementation is generic for loopback UDP, not tailored to `netperf`.

## Validation record

Validation used the unchanged RV evaluator:

```text
timeout 900 ./run-eval.sh rv
```

Result:

- Previous trusted RV result: 134 pass-like / 24 fail-like / 10 skipped.
- Current clean RV result: 142 pass-like / 16 fail-like / 10 skipped.
- Groups started and ended: 24 / 24.
- Removed failure markers: `netperf UDP_STREAM`, `netperf TCP_STREAM`,
  `netperf UDP_RR`, and `netperf TCP_RR` under both musl and glibc.
- Remaining network failures: all iperf modes and `netperf TCP_CRR` under
  both musl and glibc.
- No new fail-like markers were added by the clean RV run.
