FROM debian:bookworm-slim AS flutter
RUN apt-get update && apt-get install -y --no-install-recommends git curl ca-certificates unzip xz-utils libglu1-mesa && rm -rf /var/lib/apt/lists/*
RUN curl -fsSL -o /tmp/flutter.tar.xz https://storage.googleapis.com/flutter_infra_release/releases/stable/linux/flutter_linux_3.47.7-stable.tar.xz \
    && tar -xf /tmp/flutter.tar.xz -C /opt && rm /tmp/flutter.tar.xz \
    && git config --global --add safe.directory /opt/flutter
ENV PATH="/opt/flutter/bin:$PATH"
WORKDIR /src
COPY . .
RUN flutter pub get && flutter build web --release --base-href / \
    && rm -rf frontend/dist && mkdir -p frontend/dist && cp -r build/web/* frontend/dist/

FROM rust:1.99-bookworm AS rust
WORKDIR /src
COPY --from=flutter /src /src
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=rust /src/target/release/self26 /usr/local/bin/self26
EXPOSE 8021
VOLUME ["/data"]
ENTRYPOINT ["self26", "--data-dir", "/data"]
