#!/bin/bash
kubectl create namespace cert-manager
helm repo add jetstack https://charts.jetstack.io
helm repo update
kubectl apply -f https://github.com/jetstack/cert-manager/releases/download/v1.2.0/cert-manager.crds.yaml
helm install \
  cert-manager jetstack/cert-manager \
  --namespace cert-manager \
  --version v1.2.0 \
  --create-namespace \
  --set nodeSelector.task=infra