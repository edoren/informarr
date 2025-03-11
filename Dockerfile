FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev pkgconfig curl

RUN mkdir /app
COPY ./src /app/src/
COPY ./Cargo.toml ./Cargo.lock /app
WORKDIR /app

RUN cargo build -r

FROM alpine:latest AS runner

COPY --from=builder /app/target/release/informarr /informarr

RUN apk add --no-cache tini libgcc

WORKDIR /config

EXPOSE 3000

ENTRYPOINT ["tini", "--"]
CMD ["/informarr"]
