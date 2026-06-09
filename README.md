# lacuna

*Lacuna fills the gap between your tailnet users and the AI providers.*

Lacuna is a free and open-source API gateway for AI providers.
It is meant to be deployed in Tailscale to grant AI API access to your tailnet members without having to distribute API keys.

## ✨ Features

- **Supported Providers**: OpenAI, Anthropic, Bedrock, Gemini.
- **Automatic Routing**: Routes requests to the first compatible provider based on which endpoint is called. For example, calls to `/v1/chat/completions` are automatically routed to the first OpenAI-compatible provider.
- **Provider-Specific Routing**: Dedicated base URL for each provider. For example, calls to `/myprovider/v1/chat/completions` will always route requests to `myprovider`.
- **Prometheus Metrics**: Exposes Prometheus metrics at `/metrics` for usage monitoring. Includes per-user metrics.
- **Fine-Grained Permissions**: Control which users can access which providers and models based on Tailscale application capabilities.
- **Web Interface**: Minimal web interface that displays configured providers.

## 📋 Changelog

See the [GitHub Releases](https://github.com/Flared/lacuna/releases) page.

## 🚀 Usage

```
lacuna --config <path> [--host <host>] [--port <port>]
```

## 🔌 Using the Gateway

There are usage examples in the [examples directory](examples).

## 🐳 Docker Image

Docker images are published at [ghcr.io/flared/lacuna](https://github.com/Flared/lacuna/pkgs/container/lacuna):

```
docker pull ghcr.io/flared/lacuna:latest
```

## ⚙️ Configuration

The configuration file defines one or more AI providers in JSON format.

The provided configuration file may include environment variable substitution using the `${VAR_NAME}` syntax.

**Example Configuration**
```json
{
  "lacuna": {
    "logging": {
      "format": "console",
      "level": "info"
    },
    "capabilities_header": "Tailscale-App-Capability",
    "identity_header": "Tailscale-User-Login"
  },
  "providers": {
    "anthropic": {
      "name": "Anthropic",
      "baseurl": "https://api.anthropic.com",
      "authorization": {
        "type": "x-api-key",
        "apikey": "${ANTHROPIC_API_KEY}"
      },
      "capability": {
        "models": { 
          "claude-*": {} 
        },
        "user_agents": ["claude-code"]
      },
      "compatibility": {
        "anthropic_messages": true
      }
    },
    "bedrock": {
      "name": "AWS Bedrock",
      "baseurl": "https://bedrock-runtime.us-east-1.amazonaws.com",
      "authorization": {
        "type": "bearer",
        "apikey": "${BEDROCK_API_KEY}"
      },
      "compatibility": {
        "bedrock_model_invoke": true
      }
    }
  }
}
```

### User Capabilities

When `capabilities_header` is set, Lacuna expects a header that follows the Tailscale application capabilities format:

```json
{
  "flare.io/cap/lacuna/grants": [
      { "providers": ["firstprovider", "secondprovider"] },
      { "providers": ["thirdprovider-*"], "models": ["model-1"] }
  ],
  "flare.io/cap/lacuna/labels": [
      { "team": "platform", "env": "production" }
  ]
}
```

In the above example, the user may:
- Use all models of `firstprovider` and `secondprovider`.
- Use `model-1` in any provider that starts with `thirdprovider-`.

The `flare.io/cap/lacuna/labels` key carries flat key-value metadata that is attached to Prometheus metrics and traces for observability.

Notes:
- Providers and models may contain glob patterns.
- Empty lists and omitted values default to `["*"]`.

More on Tailscale application capabilities:
- [Tailscale application capabilities documentation](https://tailscale.com/docs/features/access-control/grants/grants-app-capabilities)
- [examples/tailscale](examples/tailscale)

### Provider Capabilities

When `capability` is set on a provider, Lacuna will use it to filter the requests that are allowed to use that provider:

```json
{
  "providers": {
    "anthropic": {
      "baseurl": "https://api.anthropic.com",
      "authorization": null,
      "capability": {
        "models": {
          "claude-*": {}
        },
        "user_agents": ["claude-code"]
      }
    },
    "bedrock": {
      "baseurl": "https://bedrock-runtime.us-east-1.amazonaws.com",
      "capability": {
        "models": {
          "us.anthropic.claude-opus-4-5*": { "rewrite": "arn:aws:bedrock:us-east-1:123456789012:application-inference-profile/abcd1234567" },
          "us.anthropic.claude-haiku-4-5*": {}
        }
      }
    }
  }
}
```

In the above example:
- The `anthropic` provider will only allow requests with a User-Agent that match Lacuna's built-in `claude-code` pattern.
- The `anthropic` provider may only be used with models that match the `claude-*` glob pattern.

`capability.models` maps each model glob pattern to a settings object with an optional `rewrite`:
- `{}` ⇒ the model is allowed and forwarded as-is.
- `{ "rewrite": "<target>" }` ⇒ the model is allowed and the upstream is called with `<target>` instead. In the example, requests for `us.anthropic.claude-opus-4-5*` are sent to Bedrock as the configured application-inference-profile ARN.

Notes:
- For rewrite resolution, the most specific matching pattern wins.
- An empty or omitted `models` allows all models (no rewrite).
- Built-in User-Agent patterns can be found in [src/user_agent.rs](src/user_agent.rs).

## 📦 Dev Dependencies

- cargo: https://doc.rust-lang.org/cargo/getting-started/installation.html
- cargo-edit: `cargo install cargo-edit`
- pnpm: https://pnpm.io/installation

## 🛠️ Contributing

**General**
- `make ci`: Run CI-equivalent locally.
- `make docker-build`: Build the Docker image.
- `bin/bump-version`: Bump the version number and allow you to release.

**API Targets**
- `make build`: Build the API.
- `make run`: Run the app with the example config.
- `make test`: Run tests.
- `make format`: Format the code.
- `make fix`: Automatically fix lint warnings.
- `make clippy`: Lint for common errors.

**Frontend Targets**
- `make frontend-build`: Build the frontend.
- `make frontend-format`: Format the frontend.
- `make frontend-lint`: Lint the frontend.
- `make frontend-run`: Serve the frontend with auto-reload using Vite. You must also have the backend running.
