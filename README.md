# Startups API — Rust (Axum + Firestore) prototype

A CRUD API for startups in Rust, using Firestore (Firebase) as the
external data infrastructure via its REST API.

## Setup

1. Copy `.env.example` to an `.env` file and fill in:
   - `FIREBASE_PROJECT_ID`: your Firebase project's Project ID
   - `FIREBASE_ACCESS_TOKEN`: generate one with `gcloud auth print-access-token`
     (expires in ~1h — regenerate whenever you test again)
      - `SERPAPI_KEY`: your SerpAPI key (https://serpapi.com), used to collect
     market/competitor data for each startup
   - `AI_API_KEY`: your Anthropic API key (https://console.anthropic.com),
     used to synthesize the report from the collected data
   - `REPORT_CACHE_TTL_HOURS`: how many hours a cached report is considered
     "fresh" before being regenerated (optional — defaults to `6` if omitted)
     
     The `firebaseConfig` object identifies which Firebase project the
frontend (index.html is only for tests) talks to:

- `apiKey`: a public identifier for your Firebase project's web API
  (not a secret — it's meant to be embedded in client-side code;
  access is still controlled by Firebase Security Rules and this
  API's own auth middleware, not by hiding this key)
- `authDomain`: the domain Firebase Authentication uses to complete
  the login flow (format: `<project-id>.firebaseapp.com`). This must
  match exactly what's registered as the redirect URI in Google
  Cloud / Azure — a mismatch here causes `redirect_uri_mismatch`
  errors during Google/Microsoft sign-in
- `projectId`: your Firebase project's unique ID, used to route
  requests to the right Firestore database

2. Run:
   ```bash
   cargo build
   cargo run
   ```

3. The API comes up at `http://localhost:3000`, with the routes:
   - `POST   /startups`
   - `GET    /startups`
   - `GET    /startups/:id`
   - `PUT    /startups/:id`
   - `DELETE /startups/:id`

## Authentication

Every route requires a valid Firebase ID Token in the header:

```
Authorization: Bearer <id-token>
```

The client (app/frontend) obtains this token after logging in (Google,
Microsoft, or email/password — they all produce the same ID Token
format, since they all go through Firebase Auth). The middleware in
`src/auth/middleware.rs`:

1. Extracts the token from the header
2. Verifies the signature against Google's public keys (cached for 1h)
3. Checks `aud`/`iss` against your `FIREBASE_PROJECT_ID`
4. If everything checks out, injects the `uid` into the request —
   handlers access it via `Extension<UsuarioAutenticado>` (the struct
   name is still in Portuguese in the code, along with the rest of
   the implementation — this README is just the English summary)

Without a valid token, the API responds with `401 Unauthorized`
**before** ever touching the repository or Firestore.

On `POST /startups`, the `uid` extracted from the token becomes the
document's `registered_by` field — the client never sends this field
directly, so there's no way to forge "who registered it".

Testing with curl (swap `<id-token>` for a real token obtained from
the app after logging in):

```bash
curl -X POST http://localhost:3000/startups \
  -H "Authorization: Bearer <id-token>" \
  -H "Content-Type: application/json" \
  -d '{ "fantasy_name": "...", "legal_name": "...", "cnpj": "...", "description": "...", "founded_date": "2023-05-10" }'
```

### Current limitations

This is a prototype — the Firestore access token is read straight
from `.env` to keep things simple. That works fine for local testing,
but has two serious limitations for production:

1. **The token expires in ~1h** — you can't leave the API running
   without renewing it manually.
2. **It isn't secure** to keep a fixed access token in an environment
   variable for long.

The right approach in production is to use a **service account**
credential (a JSON file downloaded from the Firebase Console) and
exchange it for short-lived tokens automatically. In Rust, the
[`gcp_auth`](https://docs.rs/gcp_auth) crate does exactly that: it
reads the service account JSON and manages token renewal on its own.
Swapping `FirebaseConfig` to use that crate is the natural next step
for this prototype.

## Project structure

```
src/
├── main.rs                        # builds the Axum router, wires everything together
├── config.rs                      # loads FirebaseConfig / ReportConfig from .env
├── error.rs                       # AppError — single error type, maps to HTTP status codes
├── state.rs                       # AppState — combines StartupRepository + ReportService
├── models/
│   ├── startup.rs                 # Startup entity
│   └── report.rs                  # StartupReport entity (AI market report)
├── dtos/
│   ├── startup_dto.rs             # CreateStartupRequest, UpdateStartupRequest, StartupResponse
│   └── report_dto.rs              # ReportResponse
├── firebase/
│   ├── client.rs                  # generic HTTP client for the Firestore REST API
│   └── convert.rs                 # conversion between entities and Firestore's typed JSON format
├── auth/
│   ├── firebase_verify.rs         # verifies Firebase ID Tokens against Google's public keys
│   └── middleware.rs              # Axum middleware that enforces authentication
├── repository/
│   └── startup_repository.rs      # translates domain operations into FirestoreClient calls
├── services/
│   ├── search_service.rs          # concurrent market-data lookups (SerpAPI)
│   ├── ai_service.rs              # calls the AI API to synthesize the report
│   └── report_service.rs          # orchestrates cache lookup, search, and AI generation
├── jobs/
│   └── report_refresher.rs        # background worker that keeps reports fresh
└── handlers/
    ├── startup_handler.rs         # HTTP routes for startups (CRUD)
    └── report_handler.rs          # HTTP routes for the market report
```

Each layer only knows about the one directly below it: handlers call
the repository/service, the repository calls the generic Firestore
client, and only `firebase/client.rs` knows it's actually talking to
Firestore over HTTP. That separation is what makes it possible to
swap Firestore for another database, or swap the AI/search provider,
by touching only one file instead of the whole codebase.

## The differentiator: AI-generated market report

New endpoints:
- `GET /startups/:id/report` — returns the cached report if it's
  still fresh (younger than `REPORT_CACHE_TTL_HOURS`), otherwise
  generates a new one on the spot
- `POST /startups/:id/report/refresh` — forces a new generation,
  ignoring the cache TTL

### How it works internally

1. **Concurrent market search** (`services/search_service.rs`): three
   SerpAPI searches fired in parallel with `tokio::join!` (general
   info, competitors, customer reviews) — a real-world case of I/O
   concurrency, since each search is an independent network call
2. **AI synthesis** (`services/ai_service.rs`): the collected data +
   startup info become a prompt sent to the Anthropic API
3. **Firestore cache** (`services/report_service.rs`): the report is
   stored in the `startup_reports` collection (same id as the
   startup) with `generated_at`. A call within the TTL window returns
   the cached version instead of spending AI tokens again
4. **Background worker** (`jobs/report_refresher.rs`): every 30
   minutes, it scans all startups and regenerates whichever ones have
   an expired cache — each one in its own `tokio::spawn`, so a slow
   AI call for one startup doesn't block the others

This is what gives the "always up to date" feel without calling the
AI on every single request — the user only feels the generation delay
on the first call or once the cache expires, and the worker keeps
everything fresh on its own in the background.

### Still missing for production
- Limit the worker's concurrency (today it fires one task per startup
  with no cap — with many startups, this could saturate the
  SerpAPI/AI rate limits; a `tokio::sync::Semaphore` would fix this)
- Handle SerpAPI/Anthropic rate limits with retry and backoff
- Cap prompt size if the startup list grows very large

## Test frontend

`frontend/index.html` — a simple panel (plain HTML+CSS+JS, no build
step) to test the full flow: login → token → CRUD → report. Comments
in the file itself explain the screen's design.

The Rust API now serves this file directly (via `ServeDir` in
`main.rs`) — there's no separate frontend server. This means:

- `cargo run` brings up both at once
- Open **http://localhost:3000** in the browser to load the page
  (don't open the `.html` file directly via `file://` — without a
  server behind it, the relative API path won't resolve)
- `GET /startups`, `POST /startups`, etc. still require a token; only
  the HTML/CSS/JS itself (and the login screen) are public — that's
  handled by Axum's `fallback_service`, which sits **outside** the
  auth/CORS layers applied only to the `/startups*` routes

Before running:
1. Fill in `firebaseConfig` in the `<script>` tag of
   `frontend/index.html` with your web app's data from the Firebase
   Console (Project settings → General → your apps → SDK config) —
   these are public values, different from the backend's
   `FIREBASE_ACCESS_TOKEN`
2. Run from the project root (`cargo run` from the folder that
   contains `Cargo.toml` and `frontend/`) — `ServeDir::new("frontend")`
   is a path relative to wherever the process is started from

If Firebase complains about an unauthorized domain during popup
login, add `localhost` under **Authentication → Settings →
Authorized domains**.

## Still missing for production

- Real authentication via service account (`gcp_auth`), as mentioned above
- Integration tests (mock `FirestoreClient` via a trait + dependency injection)
- Pagination in `listar_startups` (Firestore's REST API defaults to a 20-document page limit)
- Handling duplicate-CNPJ conflicts (Firestore has no native `UNIQUE` — needs a check query before create, or using the CNPJ as the document ID)
