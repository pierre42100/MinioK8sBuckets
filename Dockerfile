FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
  libssl3 \
  && rm -rf /var/lib/apt/lists/*

COPY minio-operator /usr/local/bin/minio-operator
COPY mc /usr/local/bin/mc

ENTRYPOINT ["/usr/local/bin/minio-operator"]
