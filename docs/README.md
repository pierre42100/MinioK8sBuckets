# Minio K8S bucket operator

An operator to automatically create and update S3 buckets on Minio, with their accounts.

One deployed, this tool will allow you to automatically create Minio accounts associated with buckets.


## Pre-requisites
You will need:

* `kubectl` access to the target cluster
* A running Minio instance, and especially:
    * The URL where the API of the instance can be reached
    * The root credentials


## Installation
The operator can be installed using the following commands:

```bash
kubectl apply -f https://raw.githubusercontent.com/pierre42100/MinioK8sBuckets/master/yaml/crd.yaml
kubectl apply -f https://raw.githubusercontent.com/pierre42100/MinioK8sBuckets/master/yaml/deployment.yaml
```

!!! warning "Known limitation"
    The operator install a deployment on the `default` namespace. Currently, only this namespace is supported!

## Configure instance
In order to create buckets, the operator needs to know how to reach the Minio instance.

You first need to secret similar to that one:

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: minio-root
type: Opaque
dyringData:
  accessKey: <MINIO_ROOT_ACCESS_KEY>
  secretKey: <MINIO_ROOT_SECRET_KEY>
```

Replace `<MINIO_ROOT_ACCESS_KEY>` and `<MINIO_ROOT_SECRET_KEY>` with the appropriate values.



You can then declare a Minio instance simiarl to that one:

```yaml
apiVersion: "communiquons.org/v1"
kind: MinioInstance
metadata:
  name: my-minio-instance
spec:
  endpoint: https://minio.example.com/
  credentials: minio-root
```

!!! note
    Minio itself can be located outside of the Kubernetes cluster.


## Create a bucket
You are not ready to create your first bucket!

Here is a basic bucket example:

```yaml
apiVersion: "communiquons.org/v1"
kind: MinioBucket
metadata:
  name: first-bucket
spec:
  # The name of the minio instance
  instance: my-minio-instance
  # The name of the bucket to create
  name: first-bucket
  # The name of the secret that will be created
  # by the operator which contains credentials to 
  # use to access the bucket
  secret: first-bucket-secret
```

## More complete example
Here is a more complete example that makes use of all the available options:

```yaml
apiVersion: "communiquons.org/v1"
kind: MinioBucket
metadata:
  name: my-bucket
spec:
  instance: my-minio-instance
  name: my-bucket
  secret: my-bucket-secret
  # This must be set to true to allow unauthenticated
  # access to the bucket resources. Use this to host a
  # static website for example
  anonymous_read_access: true
  # Enable versioning on the bucket => keep old versions
  # of uploaded files
  versioning: true
  # If specified, a quota will be applied to the bucket, in bytes
  quota: 1000000000
  # Prevent files from being removed from the bucket. This parameter
  # can not be changed, once the bucket has been created
  lock: true
  # Data retention policy. Versioning must be enabled to allow this
  retention:
    # The number of days data shall be kept
    validity: 100
    # compliance => nobody can bypass the policy
    # governance => users with privileges might bypass policy restrictions
    mode: compliance
```