FROM debian:bookworm-slim

COPY minio-operator /usr/local/bin/minio-operator
COPY mc /usr/local/bin/mc

ENTRYPOINT /usr/local/bin/minio-operator
