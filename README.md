# Ječná Proxy

A proxy server for `spsejecna.cz` that handles CORS, cookie rewriting, and link replacement. This allows you to build frontend applications that communicate with the school's system without CORS issues.

> [!WARNING]
> This program is unofficial. It has nothing to do with SPŠE Ječná and is not approved by the school management.  
> Tento program je neoficiální. Nemá nic společného s SPŠE Ječná a není schválena vedením školy.

## Features
- Proxies all requests to `https://www.spsejecna.cz`
- Handles CORS (Allow-Origin, Credentials)
- Rewrites `Set-Cookie` to work on localhost
- Rewrites redirects (Location header) and HTML body links

## Docker

### Build & Run
```bash
docker-compose up --build -d
```
The proxy will be available at `http://localhost:3000`.

## Usage

### Local Development
```bash
cargo run
# or with custom settings
PORT=8080 BASE_URL=http://mysite.com cargo run
```

### Environment Variables
| Variable | Description | Default |
|----------|-------------|---------|
| `PORT` | Port to listen on | `3000` |
| `BASE_URL` | Public URL of the proxy (e.g. `https://proxy.jecnajevecna.cz`). If not set, it defaults to the request's Host header. | `http://localhost:3000` |
| `DISABLE_WARNING` | Set to `true` or `1` to disable the "Not Official" HTML banner injected into pages. | `false` |
| `MODE` | Proxy mode. Can be `spsejecna` or `jidelna` (for canteen). | `spsejecna` |
