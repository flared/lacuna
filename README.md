# lacuna

*Lacuna fills the gap between your tailnet users and the AI providers.*

Lacuna is an open-source API gateway for AI providers (OpenAI, Anthropic, Bedrock).
It is meant to be deployed in Tailscale to grant AI API access to all your tailnet members without having to distribute API keys.

## Usage

```
lacuna --config <path> [--host <host>] [--port <port>]
```

## Configuration

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
    "identity_header": "Tailscale-User-Login"
  },
  "providers": {
    "anthropic": {
      "name": "Anthropic",
      "baseurl": "https://api.anthropic.com",
      "authorization": "x-api-key",
      "apikey": "${ANTHROPIC_API_KEY}",
      "compatibility": {
        "anthropic_messages": true
      }
    },
    "bedrock": {
      "name": "AWS Bedrock",
      "baseurl": "https://bedrock-runtime.us-east-1.amazonaws.com",
      "authorization": "bearer",
      "apikey": "${BEDROCK_API_KEY}",
      "compatibility": {
        "bedrock_model_invoke": true
      }
    }
  }
}
```

## Docker Image

```
docker pull ghcr.io/flared/lacuna:latest
```

## Dev Dependencies

```
cargo install cargo-edit
```

## Contributing

- `make build`: Build the app.
- `make run`: Run the app with the example config.
- `make test`: Run tests.
- `make format`: Format the code.
- `make fix`: Automatically fix lint warnings.
- `make clippy`: Lint for common errors.
- `make ci`: Run CI-equivalent locally.
- `make docker-build`: Build the Docker image.
- `bin/bump-version`: Bump the version number and allow you to release.
