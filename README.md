# PRISM Router

Proof of concept of a WebTransports router providing real-time forwarding of arbitrary data between multiple endpoints (typically Chrome browsers).

The implementation makes use on quinn/h3 rust libraries but with some custom patches for WebTransport support.

Check deployed demo to http://prismrouter.com

## Running locally

```
cargo r -- --key ssl.key --cert ssl.crt
```

When using prism locally with the development ssl keys including in the repo start chrome with the following arguments to accept that dev certificate.

```
--origin-to-force-quic-on=localhost:4433 --ignore-certificate-errors-spki-list=BWtnuhjDBSoJeLuR3Ko1e8BT+oFRWoF8bDaL0NW7fBA=
```
