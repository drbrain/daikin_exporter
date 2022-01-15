FROM --platform=$BUILDPLATFORM rust:1.58-slim-buster AS builder

RUN apt-get update -qq && apt-get -qqy install pkg-config libssl-dev && rm -rf /var/cache/apt/* /var/lib/apt/*

WORKDIR /work

COPY . .

RUN cargo build --release

FROM debian:buster-slim AS release

ARG GITHUB_SHA

LABEL org.label-schema.schema-version = "1.0"
LABEL org.label-schema.vcs-ref = "${GITHUB_SHA}"
LABEL org.label-schema.name = "daikin_exporter"
LABEL org.label-schema.url = "https://github.com/drbrain/daikin_exporter"
LABEL org.label-schema.vcs-url = "https://github.com/drbrain/daikin_exporter"
LABEL org.label-schema.docker.cmd = "docker run -d --network=host daikin_exporter"

RUN apt-get update -qq && apt-get -qqy install openssl && rm -rf /var/cache/apt/* /var/lib/apt/*

COPY --from=builder /work/target/release/daikin_exporter /usr/local/bin/daikin_exporter
COPY daikin.docker.toml /daikin.toml

EXPOSE 9150/tcp
EXPOSE 30051/udp

CMD ["/usr/local/bin/daikin_exporter", "daikin.toml"]
