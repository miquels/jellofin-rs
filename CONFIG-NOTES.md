# Configuration Notes

## YAML Compatibility

The Rust implementation uses `serde_yaml` 0.9, which follows YAML 1.2 specification.

### Boolean Values

**Important:** Use `true`/`false` for boolean values, not `yes`/`no`.

Go example config:
```yaml
jellyfin:
  autoregister: yes
```

Rust config (correct):
```yaml
jellyfin:
  autoregister: true
```

This is the main difference when migrating from the Go server's YAML configuration.
