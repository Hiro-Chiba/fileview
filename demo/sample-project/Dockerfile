FROM rust:1.90 AS build
WORKDIR /src
COPY . .
RUN cargo build --release -p nebula-cli

FROM debian:bookworm-slim
COPY --from=build /src/target/release/nebula /usr/local/bin/nebula
EXPOSE 8080
ENTRYPOINT ["nebula", "serve"]
