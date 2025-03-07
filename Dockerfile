FROM rust:latest AS builder

WORKDIR /app

COPY . .

RUN cargo install --path .

FROM debian:stable-slim

RUN apt-get update \
    && apt-get install -y openssl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/cargo/bin/chbot /app/chbot

ENTRYPOINT [ "/app/chbot" ]
CMD [ "--help" ]
