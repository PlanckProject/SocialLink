# UI TLS certificates

This directory is bind-mounted **read-only** into the `ui` container at `/app/certs`
(see `docker-compose.yml`). It is where the UI's HTTPS / HTTP-2 listener reads its
certificate and private key.

## Enabling HTTPS / HTTP-2

1. Put a PEM certificate chain and private key here:

   - `fullchain.pem` — the certificate (leaf + any intermediates)
   - `privkey.pem` — the matching private key

   (Or point `UI_TLS_CERT` / `UI_TLS_KEY` at different filenames.)

2. Turn TLS on and bring the stack up:

   ```bash
   UI_TLS_ENABLED=true docker compose up --build
   ```

   The UI now serves **HTTP/2** (with automatic HTTP/1.1 fallback) at
   `https://localhost:3000`. Set `UI_HTTP2=false` for HTTPS/1.1 only.

The same switches are documented as the `tls` block in
`config/<env>/server.yaml` and as `UI_*` environment variables in
`docker-compose.yml`.

## Local development certificate

For a throwaway local cert (browsers will warn — it is self-signed):

```bash
openssl req -x509 -newkey rsa:2048 -nodes -keyout privkey.pem -out fullchain.pem \
  -days 365 -subj "/CN=localhost" \
  -addext "subjectAltName=DNS:localhost,IP:127.0.0.1"
```

For a locally-trusted cert, use [mkcert](https://github.com/FiloSottile/mkcert).

## HTTP/3 (QUIC)

Node cannot terminate QUIC, so the UI never serves HTTP/3 directly. Put a QUIC-capable
reverse proxy (e.g. [Caddy](https://caddyserver.com)) in front of it: the proxy
terminates TLS and serves HTTP/2 **and** HTTP/3 to browsers while forwarding plain HTTP
to the UI. Caddy enables h3 by default; just publish port 443 over **both** TCP and UDP
(QUIC is UDP). See the "HTTPS / HTTP-2 (and HTTP-3) for the UI" section in
[`../DEVELOPMENT.md`](../DEVELOPMENT.md) for a complete Caddyfile + Compose example.

The UI's own `UI_HTTP3=true` switch only emits the `Alt-Svc` header (advertising h3 on
`UI_HTTP3_PORT`, default = the listen port); it does not serve QUIC and is unnecessary
when a proxy like Caddy fronts the UI, since Caddy advertises `Alt-Svc` itself.

> Real certificates and keys placed here are git-ignored — never commit private keys.
