# Setup for development
This guide will present you how to prepare your computer to update features of MinioK8SBucket


## Install Rust
As this project has been written using Rust, you will need to install it prior working on MinioK8SBucket. Please follow the official instructions: [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install)

## Install Minikube
First, you need to install Minikube on your computer to have a K8S environment. In order to do this, please follow the official instructions: [https://minikube.sigs.k8s.io/docs/start](https://minikube.sigs.k8s.io/docs/start)


## Start Minikube
You will then need to start Minikube using the following command:

```bash
minikube start
```

You can then make sure that Minikube is working properly:

```
minikube kubectl get nodes
```

You should get a response similar to this one:

```
NAME       STATUS   ROLES           AGE     VERSION
minikube   Ready    control-plane   2m16s   v1.32.0
```

## Clone repository
Clone this repository using:

```bash
https://gitea.communiquons.org/pierre/MinioK8sBuckets
```

!!! note "Gitea account request"
    If you want to get a Gitea account to make pull request on this repository, you will need to contact me at: `pierre.git@communiquons.org`

## Deploy Minio
First, enable Minikube tunnel:
```bash
minikube tunnel --bind-address '127.0.0.1' 
```

You will then need to deploy Minio in Minikube. Apply the Minio deployment located at the in MinioK8SBucket repository:

```bash
minikube kubectl -- apply -f yaml/minio-dev-deployment.yml
```

Wait for the pod to become ready:

```bash
minikube kubectl -- get pods -w
```

Check for the availability of the service that expose Minio to your host computer:

```bash
minikube kubectl -- get services
```

You should get a result similar to this one:

```
NAME         TYPE           CLUSTER-IP     EXTERNAL-IP   PORT(S)                         AGE
kubernetes   ClusterIP      10.96.0.1      <none>        443/TCP                         31m
minio        LoadBalancer   10.103.82.87   127.0.0.1     9000:30656/TCP,9090:31369/TCP   6m40s
```

You should be able to access minio at the following address: [http://127.0.0.1:9090](http://127.0.0.1:9090/)

Minio API should be available at: [http://127.0.0.1:9000/](http://127.0.0.1:9000/)

## Deploy CRD
You will need then to deploy the Custom Resource Definitions of MinioK8SBucket using the following command:

```bash
minikube kubectl -- apply -f yaml/crd.yaml
```

## Run operator
You can then run the project using the following command:

```bash
cargo fmt && cargo clippy && RUST_LOG=debug cargo run --
```

## Create a first bucket
You should be able to create a first bucket using the following command:

```bash
minikube kubectl -- apply -f test/test-outside-cluster.yaml
```

The bucket should then appear in buckets list:

```bash
minikube kubectl -- get buckets
```
```
NAME           AGE
first-bucket   8m43s
```

Have fun working for MinioK8SBucket!