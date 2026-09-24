# Suggested commands

```sh
make dev                         # dashboard, PORT defaults to 3003
make build
make build-release
make test
make fmt
make clippy                      # all targets/features
make check                       # all targets/features
make all                         # fmt, check, test, release build

cargo test <name-filter>         # targeted colocated Rust test
node --test tests/session-utils.test.mjs
npm ci --ignore-scripts && npm test && npm pack --dry-run --ignore-scripts
npm run check:package
```

- `PORT=3004 make dev` changes the local port.
- Linux systemd targets require `sudo`; render the source template with `make service-file`.
- On macOS, standard Git/Cargo/npm command forms need no project-specific variation.