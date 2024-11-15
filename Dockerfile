FROM rust:latest AS builder

WORKDIR /app

COPY . .

RUN cargo install --path .

FROM debian:stable-slim

RUN apt-get update \
    && apt-get install -y openssl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/cargo/bin/chbot /app/chbot

EXPOSE 3000
EXPOSE 4000

ENTRYPOINT [ "/app/chbot" ]
CMD [ "--help" ]