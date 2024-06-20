FROM rust:1.79 as builder
WORKDIR /usr/src/myapp
COPY . .
RUN cargo install --path .

FROM debian:bookworm-slim
RUN rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/froggi /usr/local/bin/froggi

EXPOSE 3000

CMD ["froggi"]