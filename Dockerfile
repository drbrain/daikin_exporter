FROM rust:1.58-alpine3.15 AS builder

RUN apk update && apk add musl-dev openssl-dev && rm -rf /var/cache/apk/*

WORKDIR /work

COPY . .

RUN cargo build --release

FROM alpine:3.15

RUN apk update && apk add libc6-compat openssl && rm -rf /var/cache/apk/*

COPY --from=builder /work/target/release/daikin_exporter /usr/local/bin/daikin_exporter
COPY daikin.docker.toml /daikin.toml

EXPOSE 9150/tcp
EXPOSE 30051/udp

CMD ["/usr/local/bin/daikin_exporter", "daikin.toml"]
