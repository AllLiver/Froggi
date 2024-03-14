FROM rust:slim

WORKDIR /usr/src/froggi
COPY . .

RUN cargo install --path .
RUN cargo clean
RUN rm -rf /usr/src/froggi/.git

EXPOSE 8080
CMD ["froggi"]