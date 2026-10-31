# syntax=docker/dockerfile:1
# check=skip=SecretsUsedInArgOrEnv
# TOKEN_USAGE_INSIGHTS_SERVICE is a mode flag, not a credential.

FROM rust:1-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /src

ARG APP_BUILD_DATE=""
ENV APP_BUILD_DATE=${APP_BUILD_DATE}

COPY Cargo.toml Cargo.lock build.rs ./
COPY src ./src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked --bin token-usage-insights \
    && install -D -m 0755 target/release/token-usage-insights /out/token-usage-insights

FROM alpine:3.22

RUN addgroup -S app \
    && adduser -S -G app -h /home/app app \
    && mkdir -p /data \
    && chown app:app /data

WORKDIR /app

COPY --from=builder /out/token-usage-insights /app/token-usage-insights
COPY static /app/static
COPY pricing.csv /app/pricing.csv

ENV INSIGHTS_DIR=/data \
    HOST=0.0.0.0 \
    PORT=3003 \
    HOME=/home/app \
    TOKEN_USAGE_INSIGHTS_SERVICE=1

USER app
VOLUME ["/data"]
EXPOSE 3003

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget -q -O /dev/null "http://127.0.0.1:${PORT}/api/version" || exit 1

# Do not copy VERSION into /app. The entrypoint disables auto-update as a second guard.
ENTRYPOINT ["/app/token-usage-insights", "--no-auto-update"]
