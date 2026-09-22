# Server Command

Start a local web server to browse and view your recipe collection.

<img width="600" alt="recipes" src="screenshots/recipe-list.png" />
<img width="600" alt="recipe" src="screenshots/recipe-detail.png" />
<img width="600" alt="shopping list" src="screenshots/shopping-list.png" />
<img width="600" alt="pantry" src="screenshots/pantry.png" />

## Usage

```
cook server [OPTIONS] [BASE_PATH]
```

## Arguments

| Argument | Description |
|----------|-------------|
| `[BASE_PATH]` | Root directory containing recipe files (default: current directory) |

## Options

| Option | Description |
|--------|-------------|
| `--host [<ADDRESS>]` | Allow connections from external hosts (default: localhost only). Optionally bind to a specific address. |
| `-p, --port <PORT>` | Port number (default: 9080) |
| `--open` | Automatically open the web interface in your default browser |
| `--cors-origin <ORIGIN>` | Origin allowed to make cross-origin browser requests. Repeatable. `*` lets any origin read, but not write. Default: none. |
| `--cors-allow-credentials` | Allow cross-origin requests to carry cookies and credentials. Requires an explicit `--cors-origin`. |
| `--no-csrf-check` | Disable same-origin enforcement on requests that modify recipes, read the cook.md sign-in state, or open the editor's websocket. |

## Examples

```bash
# Start on localhost:9080
cook server

# Serve recipes from a specific directory
cook server ~/my-recipes

# Custom port with auto-open
cook server --port 8080 --open

# Allow access from other devices on the network
cook server --host

# Let a frontend at localhost:3000 use the full API, including writes
cook server --cors-origin http://localhost:3000

# Behind a reverse proxy, name the public origin so the UI can still write
cook server --cors-origin https://cook.example.com
```

## Notes

- By default, only accepts connections from localhost
- Use `--host` on trusted networks only — recipes become accessible to anyone on the network
- By default no other website can use the server from your browser: responses carry no CORS headers, so a page on another origin cannot read them, and a cross-origin request that would modify recipes is refused with `403`. Naming origins with `--cors-origin` lets those origins read and write. `--cors-origin '*'` lets every website you visit read your recipes, pantry and shopping list, though not change them. Requests with no `Origin` header — `curl`, scripts, anything that is not a browser — are unaffected. See [the API reference](api.md).
- The cook.md sign-in state — `/api/sync/status` and the Preferences page — only answers the server's own pages and origins named with `--cors-origin`, even under `--cors-origin '*'`. While you sign in it includes the pending login code, which another site could otherwise approve with its own account.
- The editor's language-server websocket (`/api/ws/lsp`) is held to the same rule. Websockets are not covered by CORS at all, so without it any website could start a language server on your machine.
- Behind a reverse proxy that rewrites `Host`, pass `--cors-origin` with the public origin (for example `--cors-origin https://cook.example.com`). The same-origin check reads the real `Host` header and ignores `X-Forwarded-Host`, which any client can set freely.
- `--no-csrf-check` turns that same-origin enforcement off entirely, for the API, the web UI's new-recipe form, the sign-in state and the editor's websocket. Its former spelling, `--no-cors`, still works.
- The web interface supports recipe browsing, scaling, search, and shopping list management
- The UI language is negotiated per request from the browser's `Accept-Language` header — each visitor sees the interface in their own language (supported: `en-US`, `de-DE`, `nl-NL`, `fr-FR`, `es-ES`, `eu-ES`, `sv-SE`). For static sites, see the `--lang` flag of [`cook build web`](build.md#localization).
- Mobile-friendly responsive layout
