# Setup for development
This guide will present you how to prepare your computer to update features of MinioK8SBucket

## Install Minikube
First, you need to install Minikube on your computer to have a K8S environment. In order to do this, please follow the official instructions: https://minikube.sigs.k8s.io/docs/start


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

!!! warning "Gitea account request"
    If you want to get a Gitea account to make pull request on this repository, you will need to contact me at: `pierre.git@communiquons.org`

## Deploy Minio
You will then need to deploy Minio in Minikube. Apply the Minio deployment located at the in MinioK8SBucket repository:

```bash
minikube kubectl -- apply -f yaml/minio-dev-deployment.yml
```

Check for launching of pod:

```bash
minikube kubectl -- get pods -w
```