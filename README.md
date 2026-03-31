# SocialLink — a self-hostable link-in-bio platform

A refined, self-hostable "one link in bio" profile. A public, server-rendered
profile page (avatar + cover photo + bio + grouped links) that the owner edits
through a protected admin area. Links can be organized into named groups, set to
self-expire, and are tracked with per-day view/click analytics. The entire look is
configurable two ways: **static defaults in config** *and* a **runtime-editable
theme JSON** with a live admin editor and import/export.

- **Backend:** Rust (edition 2024) + [Axum](https://github.com/tokio-rs/axum), built around
  a small set of pluggable providers (database/time-series, storage, cache, logging).
- **Frontend:** [Nuxt 4](https://nuxt.com) in **SSR** (universal) mode.
- **Deploy:** Docker / Docker Compose only (Podman parity provided).

> 🛠️ **Building, configuring, deploying, or contributing?** All setup, configuration,
> architecture, and API reference lives in **[DEVELOPMENT.md](./DEVELOPMENT.md)**.

---

## Getting started

SocialLink runs as a Docker / Podman Compose stack (`ui` + `api` + MongoDB). Follow
**[DEVELOPMENT.md → Quick start](./DEVELOPMENT.md#quick-start-docker-compose)** to build
and launch it, then open <http://localhost:3000> and sign in at `/admin/login`.

---

## Features

- **Editable homepage via protected routes** — owner-only admin for profile, links,
  groups, appearance, and analytics (JWT in an httpOnly cookie, Argon2 password hashing).
- **Profile** — display name, rich bio/description, location, optional social
  links, and authenticated password changes.
- **Avatar + cover photo** — upload and update; stored on a mounted volume and served
  by the API.
- **Cover fades into the background on scroll** — a classy parallax/fade effect that
  honors `prefers-reduced-motion`.
- **Links in groups** — organize links into named lists (e.g. "Shopping",
  "Stampede products"); ungrouped links render in a default section. Groups are added
  and edited **inline**, can be reordered (↑/↓), and can be made collapsible.
- **Per-link icon or image** — each link can show an emoji icon or an uploaded image
  on its left; link order is configurable per group.
- **Validated links (https-only)** — link URLs are validated on **both** the client and
  the server: scheme-less input is upgraded to `https://`, and non-https or domain-less
  URLs are rejected, so every stored link is a safe absolute redirect target.
- **Self-expiring links** — set an `expires_at`; expired links are hidden automatically.
- **Analytics (opt-in)** — page views and per-link click counts with per-day time-series
  charts in the admin — viewable over preset windows (7d/30d/90d) or a **custom date
  range**, with a paginated per-link breakdown. The time-series event store is the **single source of truth** for
  click/view counts — they are derived from events, not a separately maintained counter.
  Public display of each counter is **off by default**; when a counter is off the value is
  omitted from the API response entirely — it is never sent to the browser (SSR payload or
  client).
- **Configurable themes** — colors, fonts, radii, layout, button/link styles, background,
  blurred primary/accent shapes, branding, and presentational feature toggles. Ships with
  a default theme plus a preset library; fully editable at runtime with **live preview**
  and **`.json` import/export** — no rebuild required.
- **Single or multi mode** — one owner, or many users each at `/:username`.

---

## Modes: single vs multi

- **`single`** (default) — one owner. The seeded admin's page is served at `/`.
  Registration is disabled.
- **`multi`** — registration/login enabled; each user's public page is at `/:username`.
  The seeded admin is created as the first user.

The mode is chosen with `application.mode` in configuration — see
[DEVELOPMENT.md → Configuration](./DEVELOPMENT.md#configuration).

---

## Theming

SocialLink is fully themeable **at runtime** — colors, fonts, corner radii, layout,
button/link styles, backgrounds (including generated blurred primary/accent shapes),
branding, cover/scroll effects, and presentational feature toggles. It ships with a
default "Midnight" theme plus a preset library.

Edit everything under **Admin → Appearance** with **live preview**, keep multiple named
presets, activate one, and **import/export** a theme as a `.json` file — all with no
rebuild. Themes are applied at SSR time via server-injected CSS variables, so there is no
flash of unstyled content.

For the full theme schema, the two-layer defaults/override model, and how it is applied,
see [DEVELOPMENT.md → Theming](./DEVELOPMENT.md#theming).

---

## License

SocialLink is source-available and **free for any noncommercial purpose** under the
[PolyForm Noncommercial License 1.0.0](./LICENSE).

**Commercial use requires prior written permission.** You must obtain a separate
commercial license *before* any commercial use ("take permission, then use
commercially"). The owners — jointly, **all contributors to this repository** — have
sole and absolute discretion over whether, to whom, and on what terms commercial use is
allowed. To request commercial permission, open an issue at
<https://github.com/PlanckProject/SocialLink/issues> or contact the repository
maintainers. See [LICENSE](./LICENSE) for the full terms.
