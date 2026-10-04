# Signet Protocol dedicated server. Build it from the repository root:
#   docker build -f servidor/Dockerfile -t signet-server .
#   docker run -d --name signet --restart unless-stopped -p 7777:7777 -e SIGNET_WORLD=signet:nexo signet-server
# The world is chosen with SIGNET_WORLD: signet:nexo (native), doom:E1M1 (default) or openarena:<map>.

FROM rust:1-slim-bookworm AS build
WORKDIR /src
COPY Cargo.toml ./
COPY crates ./crates
COPY servidor ./servidor
RUN cargo build --release -p signet-server

FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends curl ca-certificates unzip \
 && rm -rf /var/lib/apt/lists/*
WORKDIR /srv/signet
# Official Doom 1.9 shareware (id Software, redistributable unmodified).
RUN mkdir -p datos/doom \
 && curl -fsSL -o /tmp/doom.zip "https://archive.org/download/DoomsharewareEpisode/DoomV1.9sw1995idSoftwareInc.action.zip" \
 && unzip -j /tmp/doom.zip DOOM1.WAD -d datos/doom \
 && echo "f0cefca49926d00903cf57551d901abe  datos/doom/DOOM1.WAD" | md5sum -c - \
 && rm /tmp/doom.zip
# Official OpenArena 0.8.8 (GPL). Only the map package is needed.
RUN mkdir -p datos/openarena/openarena-0.8.8/baseoa \
 && curl -fsSL -o /tmp/oa.zip "https://download.tuxfamily.org/openarena/rel/088/openarena-0.8.8.zip" \
 && echo "5a8faf7f5b51f351b0a1618c06b6b98a5f1a6758f1d39818de2c87df2a0bac4a  /tmp/oa.zip" | sha256sum -c - \
 && unzip -j /tmp/oa.zip openarena-0.8.8/baseoa/pak1-maps.pk3 -d datos/openarena/openarena-0.8.8/baseoa \
 && rm /tmp/oa.zip
COPY --from=build /src/target/release/servidor_dedicado /usr/local/bin/servidor_dedicado
ENV SIGNET_WORLD=doom:E1M1
EXPOSE 7777
CMD ["servidor_dedicado"]
