# Sets the base image to the official Rust slim image
FROM rust:slim

# Sets the image's working directory and copies the source code to the it
WORKDIR /usr/src/froggi
COPY . .

# Compiles and installs the source code
RUN cargo install --path .

# Cleans up build dependancies and removes the unneeded .git directory
RUN cargo clean
RUN rm -rf /usr/src/froggi/.git

# Installs nano for debugging and editing configs
RUN apt-get update -y && apt-get install -y nano

# Exposes the server's port and runs the server
EXPOSE 8080
CMD ["froggi"]