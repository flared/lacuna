# lacuna

*Lacuna fills the gap between your tailnet users and the AI providers.*

Lacuna is an open-source API gateway for AI providers (OpenAI, Anthropic, Bedrock).
It is meant to be deployed in Tailscale to grant AI API access to all your tailnet members without having to distribute API keys.

## Dev Dependencies

```
cargo install cargo-edit
```

## Contributing

- `make build`: Build the app.
- `make test`: Run tests.
- `make ci`: Run CI-equivalent locally.
- `make docker-build`: Build the Docker image.
- `bin/bump-version`: Bump the version number and allow you to release.
