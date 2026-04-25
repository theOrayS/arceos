# Network Interfaces

Read this document when changing sockets, TCP/UDP, network device integration,
socket readiness, or network-related LTP behavior.

Also read:

- `../policies/errno.md`
- `ipc-sync.md` for socket readiness, select/poll/epoll interaction, and
  blocking rules.
- `time-system-runtime.md` for timer behavior used by network timeouts.

## Workloads

- `iperf`, script `scripts/iperf/iperf_testcode.sh`: TCP/UDP throughput,
  concurrent connections, reverse transfer.
- `netperf`, script `scripts/netperf/netperf_testcode.sh`: `UDP_STREAM`,
  `TCP_STREAM`, `UDP_RR`, `TCP_RR`, `TCP_CRR`.
- LTP networking: prioritize `net.ipv6`, `net.multicast`, `net.tcp_cmds`, and
  `net_stress.*`.

## Authority Model

- socket objects own protocol state, buffers, local/remote addresses, and
  readiness.
- fd slots reference socket open-file descriptions; fd logic does not own
  protocol state.
- network device configuration belongs to the network stack or device layer,
  not the syscall dispatcher.

## Required Semantics

- unsupported address families, socket types, and protocol options return
  explicit errno, usually `EAFNOSUPPORT`, `EPROTONOSUPPORT`, `EINVAL`, or
  `ENOPROTOOPT`.
- socket readiness integrates with `select`, `poll`, and `epoll`.
- blocking socket operations must not hold high-level process/fd locks.
- TCP and UDP behavior should be verified independently before combined stress
  tests.
- IPv6, multicast, and network stress tests are later gates and should not be
  passed through fake success.

## Promotion Gates

- iperf TCP and UDP basic throughput complete before concurrent/reverse modes.
- netperf stream tests complete before request/response tests.
- LTP network runtests are promoted by family: TCP commands, IPv6, multicast,
  then stress.
