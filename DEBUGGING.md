# Debugging

## Enabling QUIC/quinn logs

```
export RUST_LOG="trace"
```

## Capturing quic packets

Enable SSL KeyLog with the following env var and configure it in wireshark SSL preferences afterwards.

```
export SSLKEYLOGFILE=/Users/gustavo.garcia/projects/anarchyco/octopus/octopus/keylog.pcap
```
