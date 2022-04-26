FROM rust:1.60
ARG RGAPI_KEY
ENV RGAPI_KEY $RGAPI_KEY
WORKDIR /usr/src/data-collection
COPY . .
RUN cargo install --path .
CMD ["data-collection"]