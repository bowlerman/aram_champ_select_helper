FROM rust as planner
WORKDIR /app
RUN cargo install cargo-chef
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM rust as cacher
WORKDIR /app
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json /app/recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM rust AS builder
WORKDIR /app
COPY --from=cacher /usr/local/cargo /usr/local/cargo
COPY --from=cacher /app/target /app/target
COPY . /app
ARG RGAPI_KEY
ENV RGAPI_KEY $RGAPI_KEY
RUN cargo build --release

FROM gcr.io/distroless/cc-debian11 AS final
WORKDIR /app
COPY --from=builder /app/target/release/data-collection /app/data-collection
CMD ["./data-collection"]