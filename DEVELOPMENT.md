# SocialLink — Development & Operations

Everything you need to build, configure, run, and extend SocialLink. For a product
overview and the full feature list, see **[README.md](./README.md)**.

## Tech stack

- **Backend:** Rust (edition 2024) + [Axum](https://github.com/tokio-rs/axum), built around a
  small set of pluggable providers (database/time-series, storage, cache, logging) — see
  [Backend architecture](#backend-architecture) below.
- **Frontend:** [Nuxt 4](https://nuxt.com) in **SSR** (universal) mode.
- **Deploy:** Docker / Docker Compose only (Podman parity provided). There is no supported
  non-Docker development or runtime workflow — see [Quick start](#quick-start-docker-compose).

---

## Repository layout

```
api/                 Rust + Axum backend
  src/
    config/          strict YAML config loader (server.yaml + social-link.yaml)
    providers/       provider traits + concrete implementations, one dir per capability:
                       cache/ (none, in_process, redis), logging/ (local rotating file),
                       storage/ (local), database/ (mongo), timeseries/ (mongo)
    auth/            Argon2 + JWT + cookie middleware
    domain/          person, link_group, link, analytics event, theme — the final
                       domain model (supersedes the legacy `models/` types)
    io/http/         router + public & admin handlers + DTOs + request/response middleware
  theme.json         default "Midnight" theme (seeded on first run)
  Containerfile      multi-stage musl build (rustls; no OpenSSL); bakes in config/
ui/                  Nuxt 4 SSR frontend
  app/
    app.config.ts    typed Theme schema + default theme
    config/themes.ts named presets
    pages/           public profile + admin/*
    components/      ProfileHeader, CoverImage, LinkGroup, LinkCard, charts, admin/*
    composables/     useApi, useAuth, useTheme, useScrollFade
    stores/          auth, config (Pinia)
  nuxt.config.ts     ssr:true + /api and /uploads proxies
  Containerfile      node:24 build -> SSR node server
config/
  <env>/server.yaml         server, logging, auth, database, timeseries, storage, cache, tls
  <env>/social-link.yaml    application mode, admin bootstrap, uploads, themes
docker-compose.yml   ui + api + mongo:7 (+ optional redis profile) + named volumes (Docker & Podman)
```

---

## Quick start (Docker Compose)

```bash
# from the repo root
docker compose up --build
```

Or use the convenience script, which auto-detects `docker compose` /
`docker-compose` / `podman compose`, builds, and starts everything detached:

```bash
chmod +x run.sh      # first time only
./run.sh             # build + up -d   (./run.sh --help for options)
./run.sh logs -f     # follow logs
./run.sh down        # tear down
```

Then open <http://localhost:3000>. Sign in to the admin at
<http://localhost:3000/admin/login> with the seeded credentials from
`config/<env>/social-link.yaml` (`admin.username`, default `admin`). Set the three
required secrets when you start the stack — for a quick local run, for example:

```bash
ADMIN_PASSWORD=admin123                 # example only; weak — change before deploying
JWT_SECRET=$(openssl rand -hex 32)      # signs auth JWTs
IP_HASH_SALT=$(openssl rand -hex 16)    # salts hashed visitor IPs
```

> [!CAUTION]
> **Change the admin password.** `admin123` is a throwaway example — anyone who reaches
> the admin login with a default or example password gains full control of the page. Set
> a strong, unique `ADMIN_PASSWORD` before exposing the server to anyone else, and change
> it immediately after your first sign-in.

> [!WARNING]
> **Change all the defaults before deploying.** Set `JWT_SECRET`, `IP_HASH_SALT`, and
> `ADMIN_PASSWORD` (and `cookie_secure: true` behind HTTPS). These three are **required**:
> the API refuses to start if they resolve to an empty value. See
> [Configuration](#configuration) below.

Compose starts three core services (plus an optional `redis` profile):

| Service | Purpose | Exposed |
|---------|---------|---------|
| `ui`    | Nuxt SSR node server | `http://localhost:3000` |
| `api`   | Axum API + `/uploads` static files | internal (proxied by `ui`) |
| `mongo` | MongoDB 7 — database *and* time-series event store | internal |
| `redis` | Optional cache backend (`--profile redis`) | internal |

The browser only talks to `ui`. Nuxt's Nitro server proxies `/api/**` and
`/uploads/**` to the `api` service, so the API is never exposed publicly. The proxy
forwards upstream redirects untouched, so the `/l/:id` click redirect lands the visitor
on the link's real destination.

Data persists in three named volumes: `mongo-data` (database + time-series events),
`uploads` (avatars/covers/link icons), and `logs` (rotating log files).

### Overriding configuration

The API reads its settings from YAML files under `config/<env>/` (baked into the image,
see [Configuration](#configuration)); Compose only injects the handful of secrets that
are interpolated into those files at load time, plus the environment selector:

```bash
SOCIAL_LINK_ENV=local \
JWT_SECRET="$(openssl rand -hex 32)" \
IP_HASH_SALT="$(openssl rand -hex 16)" \
ADMIN_PASSWORD='a-strong-password' \
docker compose up --build
```

These can also be placed in a root `.env` file read by Compose. To change anything else
(ports, CORS origins, logging, cache mode, storage paths, etc.), edit the relevant
`config/<env>/*.yaml` file — there is no environment-variable override for those
settings other than the `${VAR}` placeholders the file itself declares.

---

## Configuration

All backend configuration lives in strict, statically-typed YAML files under
`config/<env>/` (baked into the API image at build time — see the `Containerfile`), not
environment variables. The **`SOCIAL_LINK_ENV`** environment variable selects which
directory is loaded (`config/<env>/`); it **defaults to `local`** when unset. There is no
`.env.example` and no other supported way to configure the backend.

Two files are loaded per environment:

- **`config/<env>/server.yaml`** — `server` (host/port/CORS), `logging`, `auth`,
  `database`, `timeseries`, `storage`, `cache`, and `tls` (UI HTTPS / HTTP-2).
- **`config/<env>/social-link.yaml`** — `application` (mode), `admin` (bootstrap
  account), `uploads`, and `themes`.

Both files are parsed with **`deny_unknown_fields`** schemas, so a typo'd or stray key is
a hard startup error, not a silently ignored setting. Values are further validated
(nonzero ports/TTLs/limits, `/`-prefixed route prefixes, required sub-config when a
provider is selected, etc.) before the process starts.

### `${ENV_VAR}` interpolation

Anywhere in either YAML file, `${SOME_VAR}` is replaced with the value of the `SOME_VAR`
process environment variable **before** YAML parsing. This is the only supported way to
inject secrets into config — there are no per-key environment-variable overrides. A
referenced variable that isn't set is a **hard load error** naming the file and the
variable; there is no silent fallback. Use the shell-style `${SOME_VAR:-default}` form to
supply an explicit fallback that is used when the variable is unset or empty (for example,
`jwt_ttl_hours: "${JWT_TTL_HOURS:-12}"` defaults the auth-token lifetime to 12 hours).
Three variables are effectively required because
the shipped `config/local/*.yaml` reference them and the corresponding fields must be
non-empty:

| Variable | Used for |
|----------|----------|
| `JWT_SECRET` | `auth.jwt_secret` — signs auth JWTs. |
| `IP_HASH_SALT` | `auth.ip_hash_salt` — salts hashed visitor IPs for analytics. |
| `ADMIN_PASSWORD` | `admin.password` — seeded admin account password. |

Optional overrides declared by the shipped config (a default is applied when unset):

| Variable | Used for | Default |
|----------|----------|---------|
| `JWT_TTL_HOURS` | `auth.jwt_ttl_hours` — how long an issued auth JWT (and its cookie) stays valid. | `12` |

### `server.yaml`

```yaml
server:
  host: 0.0.0.0
  port: 3001
  cors_origins:
    - http://localhost:3000

logging:
  provider: local            # only supported provider today
  level: info                # base tracing level
  directives:                # extra per-target directives (YAML list, not a comma string)
    - "info"
    - "tower_http=info"
    - "axum=info"
  format: text                # text | json
  config:
    file: /data/logs/social-link.log
    mirror_stdout: true        # also write to stdout (container logs)
    rotation:
      max_size_mb: 100         # roll over after this size
      max_files: 5              # keep this many rotated generations

auth:
  jwt_secret: ${JWT_SECRET}
  jwt_ttl_hours: "${JWT_TTL_HOURS:-12}"
  cookie_secure: false
  ip_hash_salt: ${IP_HASH_SALT}

database:
  provider: mongo             # only supported provider today
  config:
    host: mongo
    port: 27017
    db: social-link
    # connection_string: ******cluster/actual-db   # when set, this
    #   is used exactly as given (including its own database) and every discrete field
    #   above (host/port/db/username/password/certificate) is ignored — the connection
    #   string alone must carry the target database.

timeseries:
  provider: mongo             # analytics/event store; only supported provider today
  config:
    host: mongo
    port: 27017
    db: social-link
    collection: events
    # connection_string works the same way here: it must carry its own database and
    #   discrete host/port/db/username/password/certificate fields are ignored when set.

storage:
  provider: local             # only supported provider today; writes to a mounted volume
  config:
    base_path: /data/uploads
    route_prefix: /uploads    # fixed application route proxied by the UI

cache:
  provider: none               # none (default) | in_process | redis
  ttl_seconds: 259200          # 3 days; applies to whichever provider is active
  # config:                    # nested, provider-specific block; shape depends on `provider`
  #   max_entries: 10000        # in_process only
  #   connection_string: redis://redis:6379   # redis only
  #   key_prefix: social-link                 # redis only

tls:                           # HTTPS / HTTP-2 termination for the UI (see below)
  enabled: "${UI_TLS_ENABLED:-false}"
  cert_path: "${UI_TLS_CERT:-/app/certs/fullchain.pem}"
  key_path: "${UI_TLS_KEY:-/app/certs/privkey.pem}"
  http2: "${UI_HTTP2:-true}"      # h2 with automatic HTTP/1.1 fallback
  http3: "${UI_HTTP3:-false}"     # advertise h3 via Alt-Svc (needs a QUIC proxy)
```

Notes:

- **`storage.config.route_prefix`** must resolve to `/uploads` (a trailing slash is
  normalized) because that is the stable application-owned route proxied by the UI.
  Raster image uploads are served with `nosniff`; SVG uploads are rejected.
- **`database.config.connection_string`**, when set (non-empty), is used verbatim and
  **ignores every discrete field** — `host`/`port`/`db`/`username`/`password`/
  `certificate` are all bypassed, so the connection string itself must carry the target
  database name. The same rule applies to `timeseries.config.connection_string`. `database`
  and `timeseries` currently both point at the same MongoDB deployment (separate logical
  collections), but are configured and connected independently so they can be pointed at
  different clusters later.
- **Cache** is selected by `cache.provider` (default `none`, caching disabled).
  `in_process` and `redis` each take a nested `cache.config` block, validated strictly
  against that provider's own shape once the provider is known — `in_process` requires
  `max_entries`; `redis` requires `connection_string` and `key_prefix`. Even `none`
  strictly rejects unexpected keys in `cache.config` (an absent block is treated as
  empty). `ttl_seconds` applies to whichever provider is active.
- **Logging** always writes to the configured `file` with size-based rotation
  (`rotation.max_size_mb`) and keeps `rotation.max_files` rotated generations (default: a
  100MB file, 5 retained generations); `mirror_stdout` additionally streams the same
  records to stdout for `docker compose logs`. `format` selects `text` or structured
  `json` output; `level` plus the `directives` list control the `tracing` filter.
- **`tls`** configures **HTTPS / HTTP-2 for the UI**, not the API (the API is only
  reached internally through the UI proxy and does not terminate TLS). It is optional
  and disabled by default. Every field is an interpolated switch that mirrors the
  `UI_TLS_*` / `UI_HTTP*` environment variables the `ui` container actually reads (the
  UI is a separate Nitro node server configured by env, not by this YAML), so the two
  stay in sync. When `enabled` is true, `cert_path` and `key_path` are required. See
  [HTTPS / HTTP-2 (and HTTP-3) for the UI](#https--http-2-and-http-3-for-the-ui).

### `social-link.yaml`

```yaml
application:
  mode: single                # single | multi

admin:
  username: admin
  email: admin@example.com
  password: ${ADMIN_PASSWORD}
  display_name: Your Name

uploads:
  max_mb: 8

themes:
  seed_file: /app/theme.json
  max_preset_per_user: 5
  max_custom_per_user: 5
```

Frontend (`ui`) configuration remains environment-variable driven (Nuxt/Nitro
convention), set via Compose:

| Variable | Default | Description |
|----------|---------|--------------|
| `API_INTERNAL` | `http://api:3001` | Server-side URL the Nitro proxy forwards to. |
| `API_BASE` | *(empty)* | Public API base for the browser; empty means relative `/api` (recommended). |
| `HOST` / `PORT` | `0.0.0.0` / `3000` | Node SSR server bind. |
| `UI_TLS_ENABLED` | `false` | Terminate TLS at the UI (serve `https`). When `false`, plain `http`. |
| `UI_TLS_CERT` | `/app/certs/fullchain.pem` | Path (in-container) to the PEM certificate chain. |
| `UI_TLS_KEY` | `/app/certs/privkey.pem` | Path (in-container) to the PEM private key. |
| `UI_HTTP2` | `true` | Serve **HTTP/2** (h2) when TLS is on; `false` = HTTPS/1.1 only. |
| `UI_HTTP3` | `false` | Advertise **HTTP/3** via `Alt-Svc` (requires an external QUIC terminator). |
| `UI_HTTP3_PORT` | *(listen port)* | UDP port advertised in the h3 `Alt-Svc` header. Only used when `UI_HTTP3=true`. |
| `UI_CERT_DIR` | `./certs` | Host directory bind-mounted read-only to `/app/certs`. |

### HTTPS / HTTP-2 (and HTTP-3) for the UI

The UI is served by a small launcher (`ui/serve.mjs`) that wraps Nuxt's Nitro request
handler (built with `nitro.preset = 'node'`) in a Node HTTP server. Depending on the
`UI_*` variables above it serves one of:

- **plain HTTP/1.1** (default, TLS off) — unchanged from before;
- **HTTPS with HTTP/2** (`UI_TLS_ENABLED=true`) — `h2` over TLS with automatic
  **HTTP/1.1 fallback** (via ALPN), so older clients and health checks still work;
- **HTTPS/1.1 only** (`UI_TLS_ENABLED=true`, `UI_HTTP2=false`).

#### Which option should I use?

Decide two things independently: **who terminates TLS** (and speaks HTTP/2 / HTTP/3 to
browsers), and **whether the built-in launcher serves TLS at all**.

| Deployment | Terminates TLS / h2 / h3 | UI settings | Notes |
|------------|--------------------------|-------------|-------|
| **Behind a CDN** (Cloudflare, Fastly, CloudFront…) | the CDN | `UI_TLS_ENABLED=true`, `UI_HTTP2=false` | Recommended for public SaaS. Browsers get h2/h3 from the edge; the origin only needs h1.1. |
| **Self-hosted edge** (Caddy / nginx / Traefik) | the proxy | `UI_TLS_ENABLED=false` | Best when you have no CDN. Caddy gives auto-TLS + h2 + **h3** in ~3 lines. |
| **Built-in TLS only** (no proxy) | `serve.mjs` | `UI_TLS_ENABLED=true`, `UI_HTTP2=true` | Simplest, dependency-free. Serves h2 + h1.1, **not** h3. |

**Recommendation:** for anything public, terminate TLS at a dedicated edge (a CDN, or a
Caddy/nginx/Traefik proxy) and let the app speak plain HTTP/1.1. An edge gives you
HTTP/3, managed certificates, and a natural place to route `/api` and add caching/WAF.
The built-in TLS mode is a fine default for smaller self-hosted installs.

#### Built-in TLS (`serve.mjs`) — minimal, no extra proxy

To turn it on, drop a certificate and key into `./certs/` (see
[`certs/README.md`](./certs/README.md)) and start the stack with TLS enabled:

```bash
# one-time throwaway self-signed cert for localhost
openssl req -x509 -newkey rsa:2048 -nodes \
  -keyout certs/privkey.pem -out certs/fullchain.pem \
  -days 365 -subj "/CN=localhost" \
  -addext "subjectAltName=DNS:localhost,IP:127.0.0.1"

UI_TLS_ENABLED=true docker compose up --build
# → UI on https://localhost:3000 (HTTP/2)
```

The certificate mount path is configured in **two** places that mirror each other: the
`tls` block of `config/<env>/server.yaml` (canonical, validated documentation) and the
`ui` service in `docker-compose.yml` (the volume mount + `UI_*` env the container
actually consumes). Point `UI_CERT_DIR` at a different host directory to use real,
CA-issued certificates. Remember to also set `auth.cookie_secure: true` in `server.yaml`
once the site is served over HTTPS.

> **Proxied requests over HTTP/2.** HTTP/2 forbids the hop-by-hop headers (`Connection`,
> `Keep-Alive`, `Transfer-Encoding`, …) that the HTTP/1.1 API legitimately returns, and
> the `/api` proxy forwards upstream response headers verbatim. The launcher strips these
> from HTTP/2 responses, so proxied calls — including the login `Set-Cookie` — succeed
> over h2. (The built-in server still cannot serve HTTP/3; use a proxy for that.)

#### Caddy (auto-TLS + HTTP/2 + HTTP/3)

Node cannot terminate QUIC in-process, so the UI never serves h3
directly. Run a QUIC-capable reverse proxy in front of it: the proxy terminates TLS and
speaks HTTP/2 **and** HTTP/3 to browsers while forwarding plain HTTP to the UI.
[Caddy](https://caddyserver.com) does this out of the box — h3 is on by default and it
manages certificates automatically. A complete example:

`Caddyfile`:

```caddy
# Terminates TLS and serves HTTP/1.1 + HTTP/2 + HTTP/3 (QUIC) for the UI.
example.com {
    reverse_proxy ui:3000
}
```

Then add a Caddy service that publishes port 443 over **both** TCP and UDP (QUIC runs
over UDP — without the UDP publish, h3 silently never works). For example, a
`docker-compose.override.yml` next to the main compose file:

```yaml
services:
  caddy:
    image: docker.io/library/caddy:2
    restart: unless-stopped
    depends_on: [ui]
    ports:
      - "80:80"
      - "443:443"       # TCP: HTTP/1.1 + HTTP/2
      - "443:443/udp"   # UDP: HTTP/3 / QUIC  <-- required for h3
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile:ro
      - caddy-data:/data
      - caddy-config:/config
volumes:
  caddy-data:
  caddy-config:
```

In this setup the UI stays plain HTTP (`UI_TLS_ENABLED=false`) because Caddy terminates
TLS, and Caddy advertises `Alt-Svc` itself, so you do **not** need `UI_HTTP3` on the UI.
Run `docker compose up --build`; browsers then reach `https://example.com` over HTTP/3,
falling back to h2/h1 automatically.

> For a purely local trial without a public domain, use a `localhost { tls internal … }`
> site block so Caddy serves h2/h3 with its own local CA (trust it once via `caddy trust`).

The UI's own `UI_HTTP3=true` switch is only for the advanced case where the UI is
exposed directly and a separate QUIC terminator listens on the advertised
`UI_HTTP3_PORT` (default: the UI's listen port). It makes the UI emit the `Alt-Svc`
header itself but still does not serve QUIC.

#### Cloudflare (or another CDN / managed edge)

With a CDN in front, the edge terminates TLS and serves HTTP/2 + HTTP/3 to browsers while
the origin only needs HTTP/1.1 — the simplest robust setup for a public deployment.

1. Point the domain at Cloudflare and set SSL/TLS mode to **Full (strict)**. Install a
   [Cloudflare Origin CA](https://developers.cloudflare.com/ssl/origin-configuration/origin-ca/)
   cert on the server as `./certs/fullchain.pem` + `./certs/privkey.pem`.
2. Serve the origin over HTTPS/1.1 (not h2):

   ```bash
   UI_TLS_ENABLED=true UI_HTTP2=false UI_HTTP3=false docker compose up --build -d
   ```

   Cloudflare talks HTTP/1.1 to origins, so origin h2 buys nothing; leaving it on is
   harmless now that the launcher strips h2-illegal headers, but off is simplest. Set
   `auth.cookie_secure: true` in `config/<env>/server.yaml` so `sl_auth` is HTTPS-only.
3. Keep `/api` **uncached** — it is dynamic and carries `Set-Cookie`. Cloudflare does not
   cache non-GET or `Set-Cookie` responses by default; don't add a blanket cache rule that
   overrides that.

> A Cloudflare Origin certificate is trusted only between Cloudflare and the origin — it
> is **not** browser-trusted and cannot be issued for a bare IP. Test the origin with
> `curl -k https://ORIGIN_IP/…` and always reach the site through the Cloudflare hostname.
> The same shape applies to any managed edge (Fastly, CloudFront, Azure Front Door).

#### nginx or Traefik

Any TLS-terminating reverse proxy works; run the UI as plain HTTP (`UI_TLS_ENABLED=false`)
behind it and set `auth.cookie_secure: true`.

nginx:

```nginx
server {
    listen 443 ssl;
    http2 on;
    server_name example.com;
    ssl_certificate     /etc/ssl/fullchain.pem;
    ssl_certificate_key /etc/ssl/privkey.pem;
    location / {
        proxy_pass http://ui:3000;
        proxy_http_version 1.1;
        proxy_set_header Host              $host;
        proxy_set_header X-Forwarded-Proto https;
        proxy_set_header X-Forwarded-For   $proxy_add_x_forwarded_for;
    }
}
```

Traefik (labels on the `ui` service; TLS via an ACME cert resolver):

```yaml
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.ui.rule=Host(`example.com`)"
      - "traefik.http.routers.ui.tls=true"
      - "traefik.http.routers.ui.tls.certresolver=le"
      - "traefik.http.services.ui.loadbalancer.server.port=3000"
```

Both serve HTTP/2 to browsers. For HTTP/3 use nginx ≥ 1.25 with `listen 443 quic
reuseport;` (open UDP/443), enable Traefik's `http3` on the `websecure` entrypoint, or
just use Caddy, which turns h3 on automatically.

---

## Backend architecture

The backend is organized around a small set of **providers** — pluggable
implementations of a capability, selected by config rather than compile-time feature
flags. Each capability has: an async trait describing the operations, one or more
concrete implementations, and a factory function, conventionally named
`get_<capability>_provider(config)`, that reads the relevant config section and returns
the selected implementation. Callers depend on the trait/enum, not on a specific
backend, so adding a new provider is additive.

Currently implemented providers (deliberately narrow — do not assume more than this).
Each lives under `api/src/providers/<capability>/` (e.g. `providers/database/mongo`,
`providers/timeseries/mongo`, `providers/cache/{none,in_process,redis}.rs`,
`providers/storage/local.rs`, `providers/logging/`):

| Capability | Trait | Implementations | Selected by |
|------------|-------|------------------|-------------|
| Database | `Database` (+ per-entity repository traits) | `mongo` (`providers/database/mongo`) | `database.provider` |
| Time-series (analytics events) | (Mongo-backed event store) | `mongo` (`providers/timeseries/mongo`) | `timeseries.provider` |
| Object storage | `ObjectStorage` | `local` (`providers/storage/local.rs`) | `storage.provider` |
| Logging | — (`tracing` subscriber init) | `local` (rotating file + optional stdout mirror, `providers/logging/`) | `logging.provider` |
| Cache | `Cache` | `none`, `in_process`, `redis` (`providers/cache/`) | `cache.provider` |

Each multi-implementation capability is modeled as a Rust enum (e.g. `CacheProvider`,
`StorageProvider`, `DatabaseProvider`) whose variants wrap the concrete implementation
and that implements the capability's async trait by dispatching to the active variant —
application code holds one `CacheProvider`/`StorageProvider`/`DatabaseProvider` value and
calls trait methods on it without knowing which backend is active. `database` and
`timeseries` are both Mongo today, but are configured and connected independently, so a
future non-Mongo time-series backend does not require touching the primary database
path.

**Extension points, not implemented:** the config shapes above intentionally leave room
for additional providers (for example, an Azure Blob Storage object storage provider, or
an OpenTelemetry/OTLP logging exporter). No such provider exists in this codebase today
— only `local` (storage/logging), `mongo` (database/timeseries), and
`none`/`in_process`/`redis` (cache) are implemented. Treat any other provider name as
unsupported until it actually lands.

### Unified response envelope

Every JSON API response is wrapped by request middleware into a single envelope shape:

```jsonc
{
  "request_id": "…",   // matches the X-Request-Id response header
  "status": 200,
  "message": "success", // or an error message on failure
  "success": true,
  "data": { /* the original handler payload, or null on failure */ }
}
```

A `X-Request-Id` (UUID v4, generated per request) is attached to the request's tracing
span and echoed back on **every** response — envelope or not — so client and server logs
can be correlated. **Exceptions:** redirects (e.g. `GET /l/:id`) and non-JSON responses
(e.g. served upload files/downloads) are passed through unmodified — no body rewriting —
but still receive the `X-Request-Id` header.

### Fresh schema, no migrations

There is a fresh MongoDB schema with **no migration framework and no legacy data
compatibility** — indexes are (re)created idempotently on boot rather than versioned.
Click/view counters are **not** stored as a mutable counter field; the time-series event
store (`timeseries.provider`) is authoritative, and counts are derived/aggregated from it
on read.

---

## Theming

There are two layers:

1. **Static defaults / schema** — `ui/app/app.config.ts` defines the typed `Theme`
   schema and the built-in default theme; `ui/app/config/themes.ts` holds named presets.
   This is the fallback base.
2. **Dynamic active theme (JSON)** — the active theme is a JSON document persisted in
   MongoDB (seeded from `api/theme.json` on first run). It is fetched at **SSR time** and
   applied via **server-injected CSS variables** (no flash of unstyled content), then
   merged over the static defaults.

Edit it at runtime under **Admin → Appearance**: form controls with live preview, manage
multiple named presets, activate one, and **import/export** the theme as a `.json` file.
`api/theme.json` and the frontend default in `app.config.ts` are kept in sync (the
"Midnight" theme).

### Theme JSON shape

```jsonc
{
  "name": "Midnight",
  "colors":     { "background", "surface", "surface_alt", "text", "text_muted",
                  "primary", "primary_contrast", "accent", "border" },
  "fonts":      { "heading", "body", "google_fonts" },
  "radius":     { "link", "link_icon", "background", "avatar", "social_icon" },
  "layout":     { "max_width", "link_style", "cover_height", "spacing", "align" },
  "button":     { "variant", "shadow", "hover_lift" },
  "background": {
    "type", "value", "gradient", "image", "overlay",
    "shapes": { "enabled", "count", "opacity", "blur", "min_size",
                "max_size", "seed" }
  },
  "effects":    { "cover_fade", "cover_parallax" },
  "branding":   { "site_name", "logo", "favicon", "footer_text", "show_footer" },
  "features":   { "show_view_count", "show_click_count", "show_cover_photo",
                  "collapsible_groups" }
}
```

All keys are `snake_case` (the API's JSON contract is snake_case throughout).
Corner radii are stored as percentage strings, then rendered as stable corner
sizes to avoid CSS percentage distortion on wide cards. Background and panel
corners are limited to `0%`–`20%`; the remaining public-page corner controls
allow `0%`–`50%`. Legacy radius scales and removed settings are converted or
dropped automatically.
See `api/theme.json` for the full default values.

---

## UI routes (Nuxt pages)

These are the **browser-facing page routes** rendered by the Nuxt SSR app (files under
`ui/app/pages/`). They are distinct from the [Backend API routes](#backend-api-routes)
below: pages render HTML, the API returns JSON. Each page fetches its data from the API
through the Nitro proxy (`/api/**` → the `api` service).

**Public**

| Route | Page file | Purpose |
|-------|-----------|---------|
| `/` | `pages/index.vue` | In **single** mode, renders the owner's public profile directly. In **multi** mode, a landing page with **Sign in** / **Create account** links. |
| `/:username` | `pages/[username].vue` | The public profile for a username in **multi** mode (e.g. `/alice`). Redirects to `/` in **single** mode. The page URL is `/:username` — **no `/u/` prefix and no `@`**; the `/u/:username` form is the backend JSON endpoint that backs it (see [Backend API routes](#backend-api-routes)), not the address-bar URL. |

> Outbound link clicks point at `/api/l/:id` (proxied to the backend `GET /l/:id`
> redirect), not a Nuxt page — see [Backend API routes](#backend-api-routes).

**Admin** (`pages/admin/*`)

All admin pages except `/admin/login` and `/admin/register` are guarded by the `auth`
middleware (`ui/app/middleware/auth.ts`): it calls `GET /api/auth/me` and, if the session
cookie is missing or invalid, redirects to `/admin/login`.

| Route | Page file | Purpose |
|-------|-----------|---------|
| `/admin` | `admin/index.vue` | Dashboard: at-a-glance view/click totals plus quick links into the other admin pages. |
| `/admin/login` | `admin/login.vue` | Sign in (public); on success redirects to `/admin`. |
| `/admin/register` | `admin/register.vue` | Create account (public) — **multi mode with `registration_enabled` only**; otherwise redirects to `/admin/login`. |
| `/admin/profile` | `admin/profile.vue` | Edit profile: display name, bio, location, socials, avatar/cover, and (multi mode) username. |
| `/admin/links` | `admin/links.vue` | Manage link groups and links: create, edit, reorder, delete. |
| `/admin/appearance` | `admin/appearance.vue` | Theme editor with live preview, named presets, and import/export (see [Theming](#theming)). |
| `/admin/analytics` | `admin/analytics.vue` | Views/clicks analytics over a selectable range. |
| `/admin/preview` | `admin/preview.vue` | Renders the owner's public page from `/api/admin/preview` **without** recording a public view. |

---

## Backend API routes

These are the **JSON HTTP endpoints** served by the Axum backend — distinct from the
[UI routes](#ui-routes-nuxt-pages) above, which render HTML. The browser calls these under
`/api/...`; Nitro strips `/api` and proxies to the Axum service (so Axum routes have **no**
`/api` prefix). Uploaded files are served at `/uploads/...`.

**Public**

- `GET /health` — liveness probe (`ok`).
- `GET /config` — mode, public feature flags, and active theme JSON.
- `GET /theme` — active theme JSON.
- `GET /profile` (single) / `GET /u/:username` (multi) — profile + ordered groups with
  active, non-expired links + ungrouped links, plus stats **only when publicly enabled**.
- `GET /l/:id` — logs a click and redirects (**303**) to the link's `https` URL.

**Auth**

- `POST /auth/login`, `POST /auth/logout`, `GET /auth/me`, `POST /auth/register`
  (multi mode only).

**Admin** (JWT cookie required, all under `/admin`)

- `GET/PUT /admin/profile`; `PUT /admin/password`;
  `POST /admin/profile/{avatar|cover}` (multipart);
  `POST /admin/uploads` — upload a link icon image (multipart).
- `GET /admin/preview` — the authenticated owner’s public page payload without
  recording a public view.
- `GET/POST /admin/groups`, `POST /admin/groups/reorder`, `PUT/DELETE /admin/groups/:id`.
- `GET/POST /admin/links`, `POST /admin/links/reorder`, `PUT/DELETE /admin/links/:id`
  (URLs validated server-side: **https-only**).
- `GET /admin/links/:id/preview` — owner-only redirect used by the preview page;
  does not increment click analytics.
- **Theme library:** `GET/POST /admin/themes`, `GET/PUT/DELETE /admin/themes/:id`,
  `POST /admin/themes/:id/activate`, `POST /admin/themes/:id/favorite`,
  `GET /admin/themes/:id/export`, `POST /admin/themes/import`, `POST /admin/presets/apply`.
- **Active theme:** `GET/PUT /admin/theme`, `GET /admin/theme/export`,
  `POST /admin/theme/import`.
- **Analytics:** `GET /admin/analytics/overview?range=7d|30d|90d|all|custom`,
  `GET /admin/analytics/links?range=…` — both accept the same `range` (default `30d`).
  For `range=custom`, also pass inclusive `start`/`end` dates as `YYYY-MM-DD`
  (e.g. `?range=custom&start=2026-01-01&end=2026-01-31`); a missing, malformed, or
  reversed custom range returns **400**.

---

## Local development (without Docker)

Not supported. Docker/Podman Compose (via `docker compose up --build` or `./run.sh`) is
the only documented development and runtime workflow — the API expects its config to be
loaded from `config/<env>/*.yaml` (baked into the container image) and its uploads/logs
directories to be the mounted `uploads`/`logs` volumes.

---

## Security notes

- `JWT_SECRET`, `IP_HASH_SALT`, and `ADMIN_PASSWORD` are **required** — config loading
  fails fast if the referenced environment variables are unset or resolve to empty
  values. Always set them to unique, random values before any real deployment.
- Set `auth.cookie_secure: true` in `config/<env>/server.yaml` when serving over HTTPS.
- Visitor IPs are never stored raw — only a salted hash is kept for analytics.
- View/click counts are **not exposed publicly unless explicitly enabled**; when a counter
  is disabled it is stripped from the API response, not merely hidden in the UI. Counts
  are derived from the time-series event store, which is the source of truth.
- Link URLs are validated server-side and must be `https` — the stored value is always a
  safe absolute redirect target.
- Keep the `api` service internal (as in the provided Compose file); the browser should
  only reach the `ui` service.
- Every response carries an `X-Request-Id` for log correlation; JSON responses are
  wrapped in a uniform success/error envelope (see [Backend architecture](#backend-architecture)).
