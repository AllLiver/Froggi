FROM rust

WORKDIR /usr/src/froggi
COPY . .

RUN cargo install --path .

EXPOSE 8080
CMD ["froggi"]