# MinioK8sBuckets

Automatically create Minio buckets based on K8S CRD.

WIP, early project

Apply all K8s config files manually:

```bash
cat yaml/*.yaml | kubectl apply -f -
```

Note : [mc tool](https://min.io/download) is required