# MinioK8sBuckets

Automatically create Minio buckets based on K8S Custom Resources.

## Installation
1. Run the following commands:
```bash
kubectl apply -f https://raw.githubusercontent.com/pierre42100/MinioK8sBuckets/master/yaml/crd.yaml
kubectl apply -f https://raw.githubusercontent.com/pierre42100/MinioK8sBuckets/master/yaml/deployment.yaml
```

2. Deploy Minio
3. Create a MinioInstance & a MinioBucket (like in [our test](test/test-inside-cluster.yaml))
4. That's it!


## Development
Apply all K8s config files manually:

```bash
cat yaml/*.yaml | kubectl apply -f -
```

Note : [mc tool](https://min.io/download) is required